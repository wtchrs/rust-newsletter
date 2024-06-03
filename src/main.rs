use newsletter_lib::configuration::get_configuration;
use newsletter_lib::startup::run;
use newsletter_lib::telemetry::{get_subscriber, init_subscriber};
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configurations = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configurations.database.with_db());

    let address = format!(
        "{}:{}",
        configurations.application.host, configurations.application.port
    );

    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}
