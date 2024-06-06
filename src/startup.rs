use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Pool, Postgres};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    pub port: u16,
    pub server: Server,
}

impl Application {
    pub async fn build(configurations: &Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(configurations);

        let sender_email = configurations
            .email_client
            .sender()
            .expect("Invalid sender email.");
        let timeout = configurations.email_client.timeout();
        let email_client = EmailClient::new(
            configurations.email_client.base_url.clone(),
            sender_email,
            configurations.email_client.authorization_token.clone(),
            timeout,
        );

        let address = format!(
            "{}:{}",
            configurations.application.host, configurations.application.port
        );

        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, email_client)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configurations: &Settings) -> Pool<Postgres> {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configurations.database.with_db())
}

fn run(
    listener: TcpListener,
    connection_pool: PgPool,
    email_client: EmailClient,
) -> std::io::Result<Server> {
    let connection_pool = web::Data::new(connection_pool);
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
