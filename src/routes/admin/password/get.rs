use crate::utils::{e500, set_flash_messages};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use tera::{Context, Tera};

pub async fn change_password_form(
    tmpl: web::Data<Tera>,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut context = Context::new();
    set_flash_messages(&mut context, flash_messages, Level::Error);

    tmpl.render("admin/password.html", &context)
        .map(|body| HttpResponse::Ok().body(body))
        .map_err(e500)
}
