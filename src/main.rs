use newsletter_lib::configuration::get_configuration;
use newsletter_lib::startup::run;
use newsletter_lib::telemetry::{get_subscriber, init_subscriber};
use sqlx::PgPool;
use std::net::TcpListener;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("newsletter".into(), "info".into());
    init_subscriber(subscriber);

    let configurations = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(&configurations.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", configurations.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}
