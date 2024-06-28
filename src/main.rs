use newsletter_lib::configuration::get_configuration;
use newsletter_lib::issue_delivery_worker::run_worker_until_stopped;
use newsletter_lib::startup::Application;
use newsletter_lib::telemetry::{get_subscriber, init_subscriber};
use std::fmt::{Debug, Display};
use tokio::task::JoinError;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configurations = get_configuration().expect("Failed to read configuration.");
    let application = Application::build(&configurations.clone()).await?;
    let application_task = tokio::spawn(application.run_until_stopped());
    let worker_task = tokio::spawn(run_worker_until_stopped(configurations));

    tokio::select! {
        result = application_task => report_exit("API", result),
        result = worker_task => report_exit("Worker", result),
    }

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!("{} has exited.", task_name),
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} has failed.",
                task_name
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} has failed to complete.",
                task_name
            )
        }
    }
}
