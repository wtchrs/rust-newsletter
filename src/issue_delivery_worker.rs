use crate::configuration::Settings;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, Postgres, Row, Transaction};
use std::time::Duration;
use tracing::field::display;
use tracing::Span;
use uuid::Uuid;

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());
    let email_client = configuration.email_client.client();

    worker_loop(connection_pool, email_client).await
}

async fn worker_loop(pool: PgPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(&pool, &email_client).await {
            Ok(ExecutionOutcome::TaskCompleted) => {}
            Ok(ExecutionOutcome::EmptyQueue) => tokio::time::sleep(Duration::from_secs(10)).await,
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id = tracing::field::Empty,
        email = tracing::field::Empty,
    )
)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    match dequeue_task(pool).await? {
        Some((mut tx, issue_id, email)) => {
            Span::current()
                .record("issue_id", &display(&issue_id))
                .record("email", &display(&email));
            send_newsletter_issue(pool, email_client, issue_id, &email).await?;
            delete_task(&mut tx, issue_id, &email).await?;
            tx.commit().await?;
            Ok(ExecutionOutcome::TaskCompleted)
        }
        None => Ok(ExecutionOutcome::EmptyQueue),
    }
}

async fn send_newsletter_issue(
    pool: &PgPool,
    email_client: &EmailClient,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    match SubscriberEmail::parse(email.to_owned()) {
        Ok(email) => {
            let issue = get_issue(pool, issue_id).await?;
            match email_client
                .send_email(
                    &email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
            {
                Err(e) => {
                    let message = "Failed to deliver issue to a confirmed subscriber. Skipping.";
                    tracing::error!(error.cause_chain = ?e,error.message = %e,message);
                    Err(e.into())
                }
                Ok(_) => Ok(()),
            }
        }
        Err(e) => {
            let message = "A confirmed subscriber's stored contact details are invalid. Skipping.";
            tracing::error!(error.cause_chain = ?e,error.message = %e,message);
            Err(e.into())
        }
    }
}

type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error> {
    let mut tx = pool.begin().await?;
    let query = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE SKIP LOCKED
        LIMIT 1
        "#,
    );
    let record = tx.fetch_optional(query).await?;
    match record {
        Some(record) => Ok(Some((
            tx,
            record.try_get("newsletter_issue_id")?,
            record.try_get("subscriber_email")?,
        ))),
        None => Ok(None),
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    tx: &mut PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE newsletter_issue_id = $1 AND subscriber_email = $2
        "#,
        issue_id,
        email
    );
    tx.execute(query).await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE newsletter_issue_id = $1
        "#,
        issue_id
    )
    .fetch_one(pool)
    .await?;

    Ok(issue)
}
