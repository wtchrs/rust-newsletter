use crate::authentication::UserId;
use crate::utils;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn admin_dashboard(
    pool: web::Data<PgPool>,
    tmpl: web::Data<tera::Tera>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(&pool, *user_id).await.map_err(utils::e500)?;

    let mut context = tera::Context::new();
    context.insert("username", &username);
    let rendered = tmpl
        .render("admin/dashboard.html", &context)
        .map_err(utils::e500)?;
    let response = HttpResponse::Ok().body(rendered);

    Ok(response)
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(pool: &PgPool, user_id: Uuid) -> Result<String, anyhow::Error> {
    let row = sqlx::query!("SELECT username FROM users WHERE user_id = $1", user_id)
        .fetch_one(pool)
        .await
        .context("Failed to fetch username.")?;
    Ok(row.username)
}
