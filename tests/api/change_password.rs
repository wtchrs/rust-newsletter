use crate::helpers::{assert_is_redirect_to, spawn_app};
use rand::distributions::Alphanumeric;
use rand::Rng;
use uuid::Uuid;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_change_password().await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_confirm": &new_password,
        }))
        .await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    // Act 1 - Login
    app.post_login(&serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    }))
    .await;

    // Act 2 - Try changing password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": app.test_user.password,
            "new_password": &new_password,
            "new_password_confirm": &another_new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Act 3 - Follow redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    // Arrange
    let app = spawn_app().await;
    let wrong_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();

    // Act 1 - Login
    app.post_login(&serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    }))
    .await;

    // Act 2 - Try changing password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &wrong_password,
            "new_password": &new_password,
            "new_password_confirm": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Act 3 - Follow redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[tokio::test]
async fn new_password_must_be_at_least_12_characters_long() {
    // Arrange
    let app = spawn_app().await;
    let invalid_new_password = generate_random_alphanumeric(11);

    // Act 1 - Login
    app.post_login(&serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    }))
    .await;

    // Act 2 - Try changing password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &invalid_new_password,
            "new_password_confirm": &invalid_new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Act 3 - Follow redirect
    let html_page = app.get_change_password_html().await;
    assert!(
        html_page.contains("<p><i>The new password must be at least 12 characters long.</i></p>")
    );
}

#[tokio::test]
async fn new_password_must_be_at_most_128_characters_long() {
    // Arrange
    let app = spawn_app().await;
    let invalid_new_password = generate_random_alphanumeric(129);

    // Act 1 - Login
    app.post_login(&serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    }))
    .await;

    // Act 2 - Try changing password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &invalid_new_password,
            "new_password_confirm": &invalid_new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // Act 3 - Follow redirect
    let html_page = app.get_change_password_html().await;
    assert!(
        html_page.contains("<p><i>The new password must be at most 128 characters long.</i></p>")
    );
}

#[tokio::test]
async fn changing_password_works() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act 1 - Login
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Act 2 - Change password
    let change_password_body = serde_json::json!({
        "current_password": app.test_user.password,
        "new_password": &new_password,
        "new_password_confirm": &new_password,
    });
    let response = app.post_change_password(&change_password_body).await;
    assert_is_redirect_to(&response, "/login");

    // Act 3 - Follow redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed successfully.</i></p>"));

    // Act 4 - Check if the user is logged out
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");

    // Act 5 - Login with new password
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": &new_password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");
}

fn generate_random_alphanumeric(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}
