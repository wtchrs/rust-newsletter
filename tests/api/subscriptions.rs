use crate::helpers::spawn_app;
use sqlx::query;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// This test is responsible for testing the /subscription endpoint.
/// It will spawn our application and then send a POST request to the /subscription endpoint.
///
/// The expected behavior of the /subscription endpoint for valid form data is as follows:
/// - The endpoint should return a 200 OK status code.
/// - The endpoint should save the subscription details in the database.
#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    let response = app.post_subscriptions_with_str(body).await;

    // Assert
    assert!(response.status().is_success());
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions_with_str(body).await;

    // Assert
    let saved = query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(app.connection_pool.as_ref())
        .await
        .expect("Failed to fetch saved subscription.");

    app.connection_pool.close().await;

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
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
    let app = &spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_subscriptions_with_str(invalid_body).await;

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
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "name is empty"),
        ("name=Ursula&email=", "email is empty"),
        ("name=&email=", "name and email are empty"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_subscriptions_with_str(body).await;

        // Assert
        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not return a 400 Bad Request when {}.",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions_with_str(body).await;

    // Assert
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions_with_str(body).await;

    // Assert
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Drop the subscriptions table to force a database error
    // sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;")
    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email;")
        .execute(app.connection_pool.as_ref())
        .await
        .expect("Failed to drop the subscriptions table");

    // Act
    let response = app.post_subscriptions_with_str(body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500);
}
