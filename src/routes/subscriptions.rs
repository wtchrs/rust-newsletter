use crate::domain::SubscriberName;
use crate::domain::{NewSubscriber, SubscriberEmail};
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(connection_pool, form),
    fields(
        email = %form.email,
        name = %form.name
    )
)]
pub async fn subscribe(
    connection_pool: web::Data<PgPool>,
    form: web::Form<FormData>,
) -> HttpResponse {
    let email = match SubscriberEmail::parse(form.0.email) {
        Ok(x) => x,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let name = match SubscriberName::parse(form.0.name) {
        Ok(x) => x,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let new_subscriber = NewSubscriber { email, name };

    match insert_subscriber(&connection_pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(connection_pool, new_subscriber)
)]
async fn insert_subscriber(
    connection_pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
