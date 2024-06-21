pub mod post;

use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages, Level};
use tera::Tera;

pub async fn login_form(
    tmpl: web::Data<Tera>,
    flash_messages: IncomingFlashMessages,
) -> HttpResponse {
    let mut context = tera::Context::new();
    let flash_messages: Vec<_> = flash_messages
        .iter()
        .filter(|m| m.level() == Level::Error)
        .map(FlashMessage::content)
        .collect();
    context.insert("flash_messages", &flash_messages);

    let rendered = tmpl
        .render("login.html", &context)
        .expect("Failed to render template.");

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(rendered)
}
