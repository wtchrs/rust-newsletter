use newsletter_lib::startup::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = std::net::TcpListener::bind("127.0.0.1:8080")?;
    run(listener)?.await
}
