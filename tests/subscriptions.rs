mod common;

use common::spawn_app;
use sqlx::query;

/// This test is responsible for testing the /subscription endpoint.
/// It will spawn our application and then send a POST request to the /subscription endpoint.
///
/// The expected behavior of the /subscription endpoint for valid form data is as follows:
/// - The endpoint should return a 200 OK status code.
/// - The endpoint should save the subscription details in the database.
#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let test_app = spawn_app().await;
    let server_address = &test_app.address;
    let connection_pool = &test_app.connection_pool;

    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Act
    let response = client
        .post(format!("{}/subscriptions", server_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(200, response.status().as_u16());

    let saved = query!("SELECT email, name FROM subscriptions")
        .fetch_one(connection_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

/// This test is responsible for testing the /subscription endpoint.
/// It will spawn our application and then send a POST request to the /subscription endpoint.
///
/// The expected behavior of the /subscription endpoint for invalid form data is as follows:
/// - The endpoint should return a 400 Bad Request status code.
/// - The endpoint should not save the subscription details in the database.
#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let server_address = &spawn_app().await.address;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(format!("{}/subscriptions", server_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not return a 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "name is empty"),
        ("name=Ursula&email=", "email is empty"),
        ("name=&email=", "name and email are empty"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not return a 400 Bad Request when {}.",
            description
        );
    }
}
