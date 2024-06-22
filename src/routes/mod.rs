mod admin;
mod health_check;
mod home;
mod login;
mod newsletters;
mod subscriptions;
mod subscriptions_confirm;

pub use admin::dashboard::admin_dashboard;
pub use health_check::health_check;
pub use home::home;
pub use login::login_form;
pub use login::post::login;
pub use newsletters::publish_newsletter;
pub use subscriptions::subscribe;
pub use subscriptions_confirm::confirm;
