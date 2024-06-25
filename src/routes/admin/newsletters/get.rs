use crate::utils::e500;
use actix_web::{web, HttpResponse};
use tera::Tera;

pub async fn publish_newsletter_form(
    tmpl: web::Data<Tera>,
) -> Result<HttpResponse, actix_web::Error> {
    tmpl.render("admin/newsletter.html", &tera::Context::new())
        .map(|body| HttpResponse::Ok().body(body))
        .map_err(e500)
}
