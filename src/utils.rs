use actix_web::http::header;
use actix_web::HttpResponse;
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages, Level};
use std::fmt::Formatter;

pub fn error_chain_fmt(e: &impl std::error::Error, f: &mut Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "{}", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

pub trait ParsingError: std::error::Error {}

impl std::error::Error for Box<dyn ParsingError> {}

pub fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub fn e400<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorBadRequest(e)
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, location))
        .finish()
}

pub fn set_flash_messages(
    context: &mut tera::Context,
    flash_messages: IncomingFlashMessages,
    level: Level,
) {
    let flash_messages: Vec<_> = flash_messages
        .iter()
        .filter(|m| m.level() >= level)
        .map(FlashMessage::content)
        .collect();
    context.insert("flash_messages", &flash_messages);
}
