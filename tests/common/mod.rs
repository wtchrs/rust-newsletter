use newsletter_lib::configuration::{get_configuration, DatabaseSettings};
use newsletter_lib::startup::run;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub connection_pool: PgPool,
    pub database: DatabaseSettings,
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let connection_string = self.database.connection_string_without_db();
        let database_name = self.database.database_name.clone();
        let connection_pool = self.connection_pool.clone();

        tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                connection_pool.close().await;
                database_clean(&connection_string, &database_name).await;
            });
        });
    }
}

pub async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let mut configurations = get_configuration().expect("Failed to read configuration.");
    configurations.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = database_configure(&configurations.database).await;

    let server = run(listener, connection_pool.clone()).expect("Failed to bind address");
    tokio::spawn(server);

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        connection_pool,
        database: configurations.database,
    }
}

async fn database_configure(db_settings: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect(&db_settings.connection_string_without_db())
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, db_settings.database_name).as_str())
        .await
        .expect("Failed to create database.");
    let connection_pool = PgPool::connect(&db_settings.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to run migrations.");

    connection_pool
}

async fn database_clean(connection_string: &str, database_name: &str) {
    let mut connection = PgConnection::connect(connection_string)
        .await
        .expect("Failed to connect to Postgres.");
    connection
        .execute(format!(r#"DROP DATABASE "{}";"#, database_name).as_str())
        .await
        .expect("Failed to drop database.");
    println!("Dropped database: {}", database_name);
}
