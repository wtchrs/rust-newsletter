pub mod post;

use actix_web::cookie::Cookie;
use actix_web::http::header::ContentType;
use actix_web::{web, HttpRequest, HttpResponse};
use tera::Tera;

pub async fn login_form(tmpl: web::Data<Tera>, request: HttpRequest) -> HttpResponse {
    let mut context = tera::Context::new();
    if let Some(error_cookie) = request.cookie("_flash") {
        context.insert("error", &error_cookie.value());
    }

    let rendered = tmpl
        .render("login.html", &context)
        .expect("Failed to render template.");

    let mut response = HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(rendered);
    // Unset the `_flash` cookie
    response
        .add_removal_cookie(&Cookie::named("_flash"))
        .expect("Failed to add removal cookie.");
    response
}
