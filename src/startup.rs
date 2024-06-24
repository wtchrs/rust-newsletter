use crate::authentication::reject_anonymous_user;
use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::*;
use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_lab::middleware::from_fn;
use secrecy::{ExposeSecret, Secret};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tera::Tera;
use tracing_actix_web::TracingLogger;

pub struct Application {
    pub port: u16,
    server: Server,
    connection_pool: web::Data<PgPool>,
}

impl Application {
    pub async fn build(configurations: &Settings) -> Result<Self, anyhow::Error> {
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

        let templates_engine = Tera::new("templates/**/*").expect("Failed to parsing templates.");

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
            templates_engine,
            configurations.application.base_url.clone(),
            configurations.application.hmac_secret.clone(),
            configurations.redis_url.clone(),
        )
        .await?;

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
pub struct HmacSecret(pub Secret<String>);

async fn run(
    listener: TcpListener,
    connection_pool: web::Data<PgPool>,
    email_client: EmailClient,
    templates_engine: Tera,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_url: Secret<String>,
) -> Result<Server, anyhow::Error> {
    let email_client = web::Data::new(email_client);
    let templates_engine = web::Data::new(templates_engine);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_url.expose_secret()).await?;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_anonymous_user))
                    .route("/dashboard", web::get().to(admin_dashboard))
                    .route("/password", web::get().to(change_password_form))
                    .route("/password", web::post().to(change_password))
                    .route("/logout", web::post().to(log_out)),
            )
            .app_data(connection_pool.clone())
            .app_data(email_client.clone())
            .app_data(templates_engine.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
