use crate::authentication::{validate_credentials, AuthError, Credentials, UserId};
use crate::routes::admin::dashboard::get_username;
use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_confirm: Secret<String>,
}

pub async fn change_password(
    pool: web::Data<PgPool>,
    session: TypedSession,
    user_id: web::ReqData<UserId>,
    form: web::Form<FormData>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();

    if form.new_password.expose_secret() != form.new_password_confirm.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    if let Err(e) = validate_new_password(form.new_password.clone()) {
        FlashMessage::error(e.to_string()).send();
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(&pool, *user_id).await.map_err(e500)?;
    let credentials = Credentials {
        username,
        password: form.current_password.clone(),
    };
    if let Err(e) = validate_credentials(&pool, credentials).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    crate::authentication::change_password(&pool, *user_id, form.new_password.clone())
        .await
        .map_err(e500)?;

    session.log_out();
    FlashMessage::info("Your password has been changed successfully.").send();
    Ok(see_other("/login"))
}

fn validate_new_password(new_password: Secret<String>) -> Result<(), anyhow::Error> {
    if new_password.expose_secret().len() < 12 {
        return Err(anyhow::anyhow!(
            "The new password must be at least 12 characters long."
        ));
    }
    if new_password.expose_secret().len() > 128 {
        return Err(anyhow::anyhow!(
            "The new password must be at most 128 characters long."
        ));
    }
    Ok(())
}
