use actix_web::web;
use newsletter_lib::configuration::{get_configuration, DatabaseSettings};
use newsletter_lib::startup::Application;
use newsletter_lib::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

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

pub struct TestApp {
    pub address: String,
    pub connection_pool: web::Data<PgPool>,
    pub database: DatabaseSettings,
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

    let configurations = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c
    };
    configure_database(&configurations.database).await;

    let application = Application::build(&configurations)
        .await
        .expect("Failed to build application.");
    let connection_pool = application.get_connection_pool();
    let address = format!("http://127.0.0.1:{}", application.port);
    tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        connection_pool,
        database: configurations.database,
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
}

async fn clean_database(connect_options: PgConnectOptions, database_name: &str) {
    let mut connection = PgConnection::connect_with(&connect_options)
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"DROP DATABASE "{}";"#, database_name).as_str())
        .await
        .expect("Failed to drop database.");
}
