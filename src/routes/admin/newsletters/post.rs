use crate::authentication::UserId;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::idempotency::{save_response, try_processing, NextAction};
use crate::utils::{e400, e500, see_other};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;
use std::fmt::{Debug, Display};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

#[tracing::instrument(
    name = "Publish a newsletter",
    skip(pool, email_client, form, user_id),
    fields(user_id = %*user_id)
)]
pub async fn publish_newsletter(
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
    form: web::Form<FormData>,
) -> Result<HttpResponse, actix_web::Error> {
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;

    let idempotency_key = idempotency_key.try_into().map_err(e400)?;
    let tx = match try_processing(&pool, &idempotency_key, &user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(tx) => tx,
        NextAction::ReturnSavedResponse(response) => {
            success_message().send();
            return Ok(response);
        }
    };

    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(&subscriber.email, &title, &html_content, &text_content)
                .await
                .context(format!(
                    "Failed to send newsletter issue to {}",
                    subscriber.email
                ))
                .map_err(e500)?,
            Err(e) => tracing::warn!("Skipping invalid subscriber email: {}", e),
        }
    }

    success_message().send();
    let response = see_other("/admin/newsletters");
    let response = save_response(tx, &idempotency_key, &user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("Newsletter has been successfully published.")
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

impl Display for ConfirmedSubscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.email)
    }
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers =
        sqlx::query!("SELECT email FROM subscriptions WHERE status = 'confirmed'")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|row| match SubscriberEmail::parse(row.email) {
                Ok(email) => Ok(ConfirmedSubscriber { email }),
                Err(error) => Err(anyhow::anyhow!(error)),
            })
            .collect();

    Ok(confirmed_subscribers)
}
