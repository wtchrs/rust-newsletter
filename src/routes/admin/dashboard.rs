use crate::session_state::TypedSession;
use actix_web::http::header;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub async fn admin_dashboard(
    pool: web::Data<PgPool>,
    tmpl: web::Data<tera::Tera>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        get_username(&pool, user_id).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((header::LOCATION, "/login"))
            .finish());
    };

    let mut context = tera::Context::new();
    context.insert("username", &username);
    let rendered = tmpl
        .render("admin/dashboard.html", &context)
        .map_err(e500)?;
    let response = HttpResponse::Ok().body(rendered);

    Ok(response)
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(pool: &PgPool, user_id: Uuid) -> Result<String, anyhow::Error> {
    let row = sqlx::query!("SELECT username FROM users WHERE user_id = $1", user_id)
        .fetch_one(pool)
        .await
        .context("Failed to fetch username.")?;
    Ok(row.username)
}
