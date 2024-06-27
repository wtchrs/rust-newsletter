use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use std::time::Duration;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Log in before post new newsletter
    app.post_login(&serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    }))
    .await;

    // Act - Post new newsletter
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // Assert
    // assert_eq!(response.status().as_u16(), 200);
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Mock is dropped here and
    // verify whether the newsletter email was not sent to the unconfirmed subscriber.
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Log in before post new newsletter
    app.post_login(&serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    }))
    .await;

    // Act - Post new newsletter
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // Assert
    // assert_eq!(response.status().as_u16(), 200);
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Mock is dropped here and
    // verify whether the newsletter email was sent to the confirmed subscriber.
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "html_content": "<p>Newsletter body as HTML</p>",
                "text_content": "Newsletter body as plain text",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            "missing content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "text_content": "Newsletter body as plain text",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            "missing html_content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "html_content": "<p>Newsletter body as HTML</p>",
                "idempotency_key": uuid::Uuid::new_v4().to_string(),
            }),
            "missing text_content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "html_content": "<p>Newsletter body as HTML</p>",
                "text_content": "Newsletter body as plain text",
            }),
            "missing idempotency_key",
        ),
    ];

    // Log in before post new newsletter
    app.post_login(&serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    }))
    .await;

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_publish_newsletter(&invalid_body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn you_must_logged_in_to_publish_a_newsletter() {
    // Arrange
    let app = spawn_app().await;

    // Act
    // Send request without login
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act 1 - Submit the newsletter form
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "html_content": "<p>Newsletter body as HTML</p>",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 2 - Follow redirect
    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>Newsletter has been successfully published.</i></p>"));

    // Act 3 - Resubmit the same form
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act 4 - Follow redirect
    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>Newsletter has been successfully published.</i></p>"));

    // Mock is dropped here and verify whether the newsletter email was sent just once.
}
