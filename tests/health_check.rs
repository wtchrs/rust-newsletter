/// This test module is responsible for testing the health_check endpoint.
/// It will spawn our application and then send a GET request to the health_check endpoint.
///
/// The expected behavior of the health_check endpoint is as follows:
/// - The endpoint is accessible at /health_check
/// - The endpoint should return a 200 OK status code and empty response body.
#[tokio::test]
async fn health_check_works() {
    // Arrange
    let server_address = spawn_app();
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", server_address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

fn spawn_app() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port  = listener.local_addr().unwrap().port();
    let server = newsletter_lib::run(listener).expect("Failed to bind address");
    tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}
