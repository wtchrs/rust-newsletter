mod admin;
mod health_check;
mod home;
mod login;
mod subscriptions;
mod subscriptions_confirm;

pub use admin::dashboard::admin_dashboard;
pub use admin::logout::log_out;
pub use admin::newsletters::publish_newsletter;
pub use admin::newsletters::publish_newsletter_form;
pub use admin::password::change_password;
pub use admin::password::change_password_form;
pub use health_check::health_check;
pub use home::home;
pub use login::login_form;
pub use login::post::login;
pub use subscriptions::subscribe;
pub use subscriptions_confirm::confirm;
