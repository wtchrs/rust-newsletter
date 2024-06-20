use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(pool, credentials))]
pub async fn validate_credentials(
    pool: &PgPool,
    credentials: Credentials,
) -> Result<uuid::Uuid, AuthError> {
    let (user_id, expected_password_hash) = match get_stored_credentials(pool, &credentials).await {
        Ok(Some((stored_user_id, stored_password_hash))) => (Some(stored_user_id), stored_password_hash),
        // For removal early return when the user is not found. This prevents timing attacks.
        _ => (None, Secret::new(
            "$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22E1AH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
                .to_string()
        ))
    };

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")
    .map_err(AuthError::UnexpectedError)??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username."))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored credentials", skip(pool, credentials))]
async fn get_stored_credentials(
    pool: &PgPool,
    credentials: &Credentials,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, AuthError> {
    let row: Option<_> = sqlx::query!(
        "SELECT user_id, password_hash FROM users WHERE username = $1",
        credentials.username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to query to retrieve stored credentials.")
    .map_err(AuthError::UnexpectedError)?
    .map(|r| (r.user_id, Secret::new(r.password_hash)));

    Ok(row)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    // Using PHC string format
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse stored password hash.")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}
