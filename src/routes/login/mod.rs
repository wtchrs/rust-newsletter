pub mod post;

use crate::utils::{e500, set_flash_messages};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use tera::Tera;

pub async fn login_form(
    tmpl: web::Data<Tera>,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut context = tera::Context::new();
    set_flash_messages(&mut context, flash_messages, Level::Info);

    tmpl.render("login.html", &context)
        .map(|body| HttpResponse::Ok().body(body))
        .map_err(e500)
}
