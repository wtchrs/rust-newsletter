use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act - 1
    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response = app.post_login(&login_body).await;

    // Assert - 1
    assert_eq!(response.status().as_u16(), 303);
    assert_is_redirect_to(&response, "/login");

    let cookies = response.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!(cookies.value(), "Authentication failed.");

    // Act - 2
    let html_page = app.get_login_html().await;

    // Assert - 2
    assert!(html_page.contains("<p><i>Authentication failed.</i></p>"));

    // Act - 3
    let html_page = app.get_login_html().await;

    // Assert - 3
    assert!(!html_page.contains("<p><i>Authentication failed.</i></p>"));
}
