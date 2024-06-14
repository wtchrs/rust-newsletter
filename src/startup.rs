use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::{confirm, health_check, publish_newsletter, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    pub port: u16,
    server: Server,
    connection_pool: web::Data<PgPool>,
}

impl Application {
    pub async fn build(configurations: &Settings) -> Result<Self, std::io::Error> {
        let connection_pool = PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy_with(configurations.database.with_db());
        let connection_pool = web::Data::new(connection_pool);

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
        let server = run(
            listener,
            connection_pool.clone(),
            email_client,
            configurations.application.base_url.clone(),
        )?;

        Ok(Self {
            port,
            server,
            connection_pool,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }

    pub fn get_connection_pool(&self) -> web::Data<PgPool> {
        self.connection_pool.clone()
    }
}

pub struct ApplicationBaseUrl(pub String);

fn run(
    listener: TcpListener,
    connection_pool: web::Data<PgPool>,
    email_client: EmailClient,
    base_url: String,
) -> std::io::Result<Server> {
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            .app_data(connection_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
