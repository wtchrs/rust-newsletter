use actix_web::{HttpResponse, Responder};

/// Check if the server is running.
/// This always returns a 200 OK status code.
///
/// # Response
///
/// - **200 OK**: The server is running.
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}
