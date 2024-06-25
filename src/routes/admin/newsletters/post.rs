use crate::authentication::UserId;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::utils::e500;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use std::fmt::{Debug, Display};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
}

#[tracing::instrument(name = "Publish a newsletter", skip(pool, email_client, form))]
pub async fn publish_newsletter(
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    _user_id: web::ReqData<UserId>,
    form: web::Form<FormData>,
) -> Result<HttpResponse, actix_web::Error> {
    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &form.title,
                    &form.html_content,
                    &form.text_content,
                )
                .await
                .context(format!(
                    "Failed to send newsletter issue to {}",
                    subscriber.email
                ))
                .map_err(e500)?,
            Err(e) => tracing::warn!("Skipping invalid subscriber email: {}", e),
        }
    }

    Ok(HttpResponse::Ok().finish())
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
