pub mod post;

use crate::startup::HmacSecret;
use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use tera::Tera;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verify(&self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(&self.tag)?;
        let query_string = format!("error={}", urlencoding::encode(&self.error));
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error.clone())
    }
}

pub async fn login_form(
    tmpl: web::Data<Tera>,
    secret: web::Data<HmacSecret>,
    query: Option<web::Query<QueryParams>>,
) -> HttpResponse {
    let error_message = match query {
        None => None,
        Some(query) => match query.verify(secret.get_ref()) {
            Ok(error_message) => Some(error_message),
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to verify query parameters using the HMAC tag."
                );
                None
            }
        },
    };

    let context = {
        let mut c = tera::Context::new();
        if let Some(error_message) = error_message {
            c.insert("error", &error_message);
        }
        c
    };

    let rendered = tmpl
        .render("login.html", &context)
        .expect("Failed to render template.");

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(rendered)
}
