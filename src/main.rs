use newsletter_lib::configuration::get_configuration;
use newsletter_lib::startup::Application;
use newsletter_lib::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configurations = get_configuration().expect("Failed to read configuration.");
    let application = Application::build(&configurations).await?;
    application.run_until_stopped().await?;

    Ok(())
}
