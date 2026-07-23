use std::env;
use anyhow::Error;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Architecture {
    pub axum: AxumConfig
}

#[derive(Deserialize, Debug)]
pub struct AxumConfig {
    pub host_addr: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SmtpConfig {
    pub address: String,
    pub username: String,
    pub password: String,
    pub from_name: String,
    pub from_email: String,
    pub reply_to_name: String,
    pub reply_to_email: String,
}

pub fn smtp_config() -> anyhow::Result<SmtpConfig> {
    let address = env::var("SMTP_ADDRESS");
    let username = env::var("SMTP_USERNAME");
    let password = env::var("SMTP_PASSWORD");

    let config_available = address.is_ok() && username.is_ok() && password.is_ok();

    if config_available {
        let from_email = env::var("SMTP_FROM_EMAIL").unwrap_or_else(|_| "team@silo.cat".to_string());
        Ok(SmtpConfig {
            address: address?,
            username: username?,
            password: password?,
            from_name: env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "Silocat".to_string()),
            from_email: from_email.clone(),
            reply_to_name: env::var("SMTP_REPLY_TO_NAME")
                .unwrap_or_else(|_| "Silocat Support".to_string()),
            reply_to_email: env::var("SMTP_REPLY_TO_EMAIL").unwrap_or_else(|_| from_email),
        })

    } else {
        Err(Error::msg("SMTP credentials are missing from the environment"))
    }
}

#[derive(Clone, Debug)]
pub struct RazorpayConfig {
    pub key_id: String,
    pub key_secret: String,
}

pub fn razorpay_config() -> anyhow::Result<RazorpayConfig> {
    let key_id = env::var("RAZORPAY_ID")?;
    let key_secret = env::var("RAZORPAY_SECRET")?;

    Ok(RazorpayConfig {
        key_id,
        key_secret,
    })
}

#[derive(Clone, Debug)]
pub struct GoogleOauthConfig {
    pub client_id: String,
    pub client_secret: String,
}

pub fn google_oauth_config() -> anyhow::Result<GoogleOauthConfig> {
    let client_id = env::var("OAUTH_ID_GOOGLE").map_err(|_| Error::msg("OAUTH_ID_GOOGLE environment variable not found"))?;
    let client_secret = env::var("OAUTH_SECRET_GOOGLE").map_err(|_| Error::msg("OAUTH_SECRET_GOOGLE environment variable not found"))?;

    Ok(GoogleOauthConfig {
        client_id,
        client_secret,
    })
}
