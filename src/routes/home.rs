use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use tera::Tera;

pub async fn home(tmpl: web::Data<Tera>) -> HttpResponse {
    let rendered = tmpl
        .render("home.html", &tera::Context::new())
        .expect("Failed to render template.");
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(rendered)
}
