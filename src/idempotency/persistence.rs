use crate::authentication::UserId;
use crate::idempotency::IdempotencyKey;
use actix_web::body::to_bytes;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
use sqlx::PgPool;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_header_pair")
    }
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: &UserId,
) -> Result<Option<HttpResponse>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"
        SELECT
            response_status_code as "response_status_code!",
            response_body as "response_body!",
            response_headers as "response_headers!: Vec<HeaderPairRecord>"
        FROM idempotency
        WHERE user_id = $1 AND idempotency_key = $2
        "#,
        **user_id,
        idempotency_key.as_ref(),
    )
    .fetch_optional(pool)
    .await?;

    match saved_response {
        None => Ok(None),
        Some(record) => {
            let status_code = StatusCode::from_u16(record.response_status_code.try_into()?)?;
            let mut response = HttpResponse::build(status_code);
            for HeaderPairRecord { name, value } in record.response_headers {
                response.append_header((name, value));
            }
            Ok(Some(response.body(record.response_body)))
        }
    }
}

pub async fn save_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: &UserId,
    http_response: HttpResponse,
) -> Result<HttpResponse, anyhow::Error> {
    let (response_head, response_body) = http_response.into_parts();
    let status_code = response_head.status().as_u16() as i16;
    let body = to_bytes(response_body)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let headers: Vec<_> = response_head
        .headers()
        .iter()
        .map(|(name, value)| HeaderPairRecord {
            name: name.to_string(),
            value: value.as_bytes().to_vec(),
        })
        .collect();

    sqlx::query_unchecked!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            response_status_code,
            response_headers,
            response_body,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, now())
        "#,
        **user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref(),
    )
    .execute(pool)
    .await?;

    let http_response = response_head.set_body(body).map_into_boxed_body();
    Ok(http_response)
}
