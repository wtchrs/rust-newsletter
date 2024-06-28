use crate::authentication::UserId;
use crate::idempotency::IdempotencyKey;
use actix_web::body::to_bytes;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
use sqlx::{Executor, PgPool, Postgres, Transaction};

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

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(HttpResponse),
}

pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: &UserId,
) -> Result<NextAction, anyhow::Error> {
    let mut tx = pool.begin().await?;
    let query = sqlx::query!(
        r#"
        INSERT INTO idempotency (user_id, idempotency_key, created_at)
        VALUES ($1, $2, now())
        ON CONFLICT DO NOTHING
        "#,
        **user_id,
        idempotency_key.as_ref(),
    );
    let n_inserted_rows = tx.execute(query).await?.rows_affected();

    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(tx))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected a saved response, but didn't find it."))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
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
    mut tx: Transaction<'static, Postgres>,
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

    let query = sqlx::query_unchecked!(
        r#"
        UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        **user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref(),
    );
    tx.execute(query).await?;
    tx.commit().await?;

    let http_response = response_head.set_body(body).map_into_boxed_body();
    Ok(http_response)
}
