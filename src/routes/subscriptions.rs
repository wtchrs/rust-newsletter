use self::SubscribeError::*;
use crate::domain::SubscriberName;
use crate::domain::{NewSubscriber, SubscriberEmail};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use crate::utils::{error_chain_fmt, ParsingError};
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

/// The form data passed to the subscribe endpoint.
/// Actix-web will automatically parse the form data into this struct.
///
/// # Fields
///
/// - `email`: The email address of the new subscriber.
/// - `name`: The name of the new subscriber.
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

/// This struct implements the [TryInto] trait,
/// which allows it to be converted into a [NewSubscriber].
/// Wrong formats of the email address or name will be caught and returned as an error.
impl TryInto<NewSubscriber> for FormData {
    type Error = Box<dyn ParsingError>;

    fn try_into(self) -> Result<NewSubscriber, Self::Error> {
        let email = SubscriberEmail::parse(self.email).map_err(Box::new)?;
        let name = SubscriberName::parse(self.name).map_err(Box::new)?;
        Ok(NewSubscriber { email, name })
    }
}

/// Add a new subscriber to the database.
///
/// # Request
///
/// ### URL-encoded Form Data
///
/// The URL-encoded form data will be passed as `form`, an instance of [FormData].
/// Every field is required.
///
/// Field   | Description
/// --------|-----------------------------------------
/// `email` | The email address of the new subscriber.
/// `name`  | The name of the new subscriber.
///
/// See [FormData] for more information.
///
/// # Response
///
/// - **200 OK** - The subscriber has been successfully added.
/// - **400 Bad Request** - The request is malformed.
/// - **500 Internal Server Error** - An error occurred while processing the request.
///
/// # Errors
///
/// This function can return [SubscribeError] which has the following variants:
///
/// - [ValidationError]: The form data is invalid.
/// - [UnexpectedError]: An error occurred while processing the request.
///
/// See [SubscribeError::status_code] for more information
/// about mapping between the error and status codes.
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(pool, email_client, base_url, form),
    fields(email = %form.email, name = %form.name)
)]
pub async fn subscribe(
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
    form: web::Form<FormData>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber = form.0.try_into().map_err(ValidationError)?;

    // Transaction start
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool.")?;
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert a new subscriber into the database.")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, &subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send the confirmation email.")?;

    Ok(HttpResponse::Ok().finish())
}

/// Errors that can occur when adding a new subscriber.
/// This is a custom error type that wraps the various errors that can occur
/// when adding a new subscriber.
#[derive(thiserror::Error)]
pub enum SubscribeError {
    /// The form data is invalid.
    #[error(transparent)]
    ValidationError(#[from] Box<dyn ParsingError>),
    /// An unexpected error occurred.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// This allows the error to be converted into an HTTP response.
/// The conversion is automatically done by `actix-web`.
impl ResponseError for SubscribeError {
    /// Maps the error to a status code.
    ///
    /// # Status Codes
    ///
    /// - [ValidationError]: 400 Bad Request
    /// - [UnexpectedError]: 500 Internal Server Error
    fn status_code(&self) -> StatusCode {
        match self {
            ValidationError(_) => StatusCode::BAD_REQUEST,
            UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl Debug for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

/// Wrapper around a [sqlx::Error] to provide a more descriptive error message.
/// This error type is used when storing the subscription token fails.
pub struct StoreTokenError(sqlx::Error);

impl Debug for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(&self, f)
    }
}

impl Display for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error occurred when storing the subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(tx, new_subscriber)
)]
async fn insert_subscriber(
    tx: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );
    tx.execute(query).await?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(tx, subscription_token)
)]
async fn store_token(
    tx: &mut Transaction<'_, Postgres>,
    subscriber_id: &Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscriber_id, subscription_token)
        values ($1, $2)
        "#,
        subscriber_id,
        subscription_token
    );
    tx.execute(query).await.map_err(StoreTokenError)?;

    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nvisit {} to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
