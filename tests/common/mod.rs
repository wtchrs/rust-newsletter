use newsletter_lib::configuration::{get_configuration, DatabaseSettings};
use newsletter_lib::startup::run;
use newsletter_lib::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
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
    pub connection_pool: PgPool,
    pub database: DatabaseSettings,
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
                database_clean(connect_options, &database_name).await;
            });
        });
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let mut configurations = get_configuration().expect("Failed to read configuration.");

    configurations.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = database_configure(&configurations.database).await;

    let sender_email = configurations
        .email_client
        .sender()
        .expect("Invalid sender email.");
    let email_client = newsletter_lib::email_client::EmailClient::new(
        configurations.email_client.base_url,
        sender_email,
        configurations.email_client.authorization_token,
    );

    let server =
        run(listener, connection_pool.clone(), email_client).expect("Failed to bind address");
    tokio::spawn(server);

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        connection_pool,
        database: configurations.database,
    }
}

async fn database_configure(db_settings: &DatabaseSettings) -> PgPool {
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

    connection_pool
}

async fn database_clean(connect_options: PgConnectOptions, database_name: &str) {
    let mut connection = PgConnection::connect_with(&connect_options)
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"DROP DATABASE "{}";"#, database_name).as_str())
        .await
        .expect("Failed to drop database.");
    println!("Dropped database: {}", database_name);
}
