use crate::errors::error_chain_fmt;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use std::fmt::{Debug, Formatter};
use uuid::Uuid;
use SubscribeConfirmError::*;

/// The query parameters for the confirm endpoint.
///
/// # Fields
///
/// - `subscription_token`: The token that was sent to the subscriber's email.
#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

/// Confirm a pending subscriber.
///
/// # Request
///
/// ### Query Parameters
///
/// The query parameters will be passed as `parameters`, an instance of [Parameters].
/// `subscription_token` field is required.
///
/// Field                | Description
/// ---------------------|---------------------------------------------------
/// `subscription_token` | The token that was sent to the subscriber's email.
///
/// See [Parameters] for more information.
///
/// # Response
///
/// - **200 OK**: The subscriber has been confirmed.
/// - **401 Unauthorized**: The token is invalid.
/// - **500 Internal Server Error**: An error occurred while processing the request.
///
/// # Errors
///
/// This function can return two types of errors:
///
/// 1. [TokenNotFoundError]
///
///    The token is invalid. It will be converted into a 401 Unauthorized response.
///
/// 2. [UnexpectedError]:
///
///    An error occurred while processing the request.
///    It will be converted into a 500 Internal Server Error response.
#[tracing::instrument(name = "Confirm a pending subscriber", skip(pool, parameters))]
pub async fn confirm(
    pool: web::Data<PgPool>,
    parameters: web::Query<Parameters>,
) -> Result<HttpResponse, SubscribeConfirmError> {
    let subscriber_id = get_subscriber_id_from_token(&pool, &parameters.subscription_token)
        .await
        .context("Failed to get subscriber ID from the database.")?;

    match subscriber_id {
        Some(id) => {
            confirm_subscriber(&pool, id)
                .await
                .context("Failed to set status `confirmed` in the database")?;
            Ok(HttpResponse::Ok().finish())
        }
        None => Err(TokenNotFoundError),
    }
}

/// The error type for the confirm endpoint.
#[derive(thiserror::Error)]
pub enum SubscribeConfirmError {
    /// The subscription token is invalid.
    #[error("Failed to find subscriber. The token is invalid.")]
    TokenNotFoundError,
    /// An error occurred while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for SubscribeConfirmError {
    fn status_code(&self) -> StatusCode {
        match self {
            TokenNotFoundError => StatusCode::UNAUTHORIZED,
            UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl Debug for SubscribeConfirmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions SET status = 'confirmed' WHERE id = $1
        "#,
        subscriber_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, subscription_token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let record = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1
        "#,
        subscription_token
    )
    .fetch_optional(pool)
    .await?;

    Ok(record.map(|r| r.subscriber_id))
}
