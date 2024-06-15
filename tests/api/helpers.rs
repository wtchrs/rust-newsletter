use actix_web::web;
use newsletter_lib::configuration::{get_configuration, DatabaseSettings};
use newsletter_lib::startup::Application;
use newsletter_lib::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sha3::Digest;
use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".into();
    let subscriber_name = "test".into();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        TestUser {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    pub async fn store(&self, pool: &PgPool) {
        let password_hash = sha3::Sha3_256::digest(self.password.as_bytes());
        let password_hash = format!("{:x}", password_hash);
        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash) VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub connection_pool: web::Data<PgPool>,
    pub database: DatabaseSettings,
    pub email_server: MockServer,
    user: TestUser,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// Extracts the confirmation links from the request to the email API.
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let email_body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link =
                reqwest::Url::parse(&raw_link).expect("Failed to parse the link.");
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&email_body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&email_body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", self.address))
            .basic_auth(&self.user.username, Some(&self.user.password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let connect_options = self.database.without_db();
        let database_name = self.database.database_name.clone();
        let connection_pool = self.connection_pool.clone();

        tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                connection_pool.close().await;
                clean_database(connect_options, &database_name).await;
            });
        });
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configurations = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };
    configure_database(&configurations.database).await;

    let application = Application::build(&configurations)
        .await
        .expect("Failed to build application.");
    let connection_pool = application.get_connection_pool();
    let address = format!("http://127.0.0.1:{}", application.port);
    let port = application.port;
    tokio::spawn(application.run_until_stopped());

    let user = TestUser::generate();
    user.store(&connection_pool).await;

    TestApp {
        address,
        port,
        connection_pool,
        database: configurations.database,
        email_server,
        user,
    }
}

async fn configure_database(db_settings: &DatabaseSettings) {
    let mut connection = PgConnection::connect_with(&db_settings.without_db())
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_settings.database_name).as_str())
        .await
        .expect("Failed to create database.");
    let connection_pool = PgPool::connect_with(db_settings.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to run migrations.");
    connection.close().await.unwrap();
    connection_pool.close().await;
}

async fn clean_database(connect_options: PgConnectOptions, database_name: &str) {
    let mut connection = PgConnection::connect_with(&connect_options)
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"DROP DATABASE "{}";"#, database_name).as_str())
        .await
        .expect("Failed to drop database.");
    connection.close().await.unwrap();
}
