use lettre::{Message, SmtpTransport, Transport};
use lettre::message::header::{ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::transport::smtp::response::Response;
use serde::Serialize;
use crate::libs::configs::SmtpConfig;

#[derive(Serialize, Debug)]
pub struct EmailData {
    pub to_name: String,
    pub to_email: String,

    pub from_name: String,
    pub from_email: String,

    pub reply_to_name: String,
    pub reply_to_email: String,

    pub email_subject: String,
    pub email_body: String,
}

pub async fn send(
    smtp_config: &SmtpConfig,
    email_data: &EmailData,
) -> anyhow::Result<Response, String> {

    println!("[DBG EMAIL SEND] 1");
    let mail_to = format!("{} <{}>", &email_data.to_name, &email_data.to_email);
    let mail_from = format!("{} <{}>", &email_data.from_name, &email_data.from_email);
    let mail_reply_to = format!("{} <{}>", &email_data.reply_to_name, &email_data.reply_to_email);
    println!("[DBG EMAIL SEND] 2");
    // construct email
    let email = Message::builder()
        .from(mail_from.parse().expect("Exception while parsing mail_from"))
        .reply_to(mail_reply_to.parse().expect("Exception while parsing mail_reply_to"))
        .to(mail_to.parse().expect("Exception while parsing mail_to"))
        .subject(&email_data.email_subject)
        .header(ContentType::TEXT_HTML)
        .body(email_data.email_body.clone())
        .expect("Exception while constructing email");
    println!("[DBG EMAIL SEND] 3");

    // parse smtp creds
    let creds = Credentials::new(smtp_config.username.clone(), smtp_config.password.clone());
    println!("[DBG EMAIL SEND] 4");
    // parse tls config
    let tls_config = match TlsParameters::builder(smtp_config.address.clone()).build() {
        Ok(tls_config) => tls_config,
        Err(err) => {
            return Err(err.to_string());
        }
    };
    println!("[DBG EMAIL SEND] 5");

    // create mailer
    let mailer = SmtpTransport::relay(
        smtp_config.address.as_str()
    )
        .expect("Exception while parsing relay_address")
        .port(587)
        .tls(Tls::Required(tls_config))
        .credentials(creds)
        .build();
    println!("[DBG EMAIL SEND] 6");

    // send email
    match mailer.send(&email) {
        Ok(response) => {
            Ok(response)
        }
        Err(err) => {
            dbg!(&err);
            Err(err.to_string())
        }
    }
}

pub async fn send_verification_email(
    smtp_config: &SmtpConfig,
    to_name: &str,
    to_email: &str,
    otp: &str,
) -> anyhow::Result<Response, String> {
    
    let email_body = format!(
        r#"
        <div style="font-family: 'Helvetica Neue', Arial, sans-serif; background-color: #18181b; color: #e4e4e7; margin: 0; padding: 40px 20px;">
            <div style="max-width: 500px; margin: 0 auto; background-color: #27272a; border-radius: 16px; border: 1px solid #3f3f46; overflow: hidden; padding: 40px 30px;">
                <div style="text-align: center; margin-bottom: 30px;">
                    <h1 style="color: #ffffff; margin: 0; font-size: 28px; font-weight: 800; letter-spacing: -0.5px;">SiloCat</h1>
                    <p style="color: #ff4655; margin: 8px 0 0; font-size: 14px; font-weight: 500; letter-spacing: 0.5px; text-transform: lowercase;">moving mountains of data, securely</p>
                </div>
                
                <div style="text-align: center;">
                    <p style="margin: 0 0 20px; font-size: 16px; color: #e4e4e7;">Hello <b>{}</b>,</p>
                    <p style="margin: 0 0 30px; color: #a1a1aa; line-height: 1.6; font-size: 15px;">Use the verification code below to complete your sign-in process.</p>
                    
                    <div style="background-color: #18181b; border: 1px solid #ff4655; border-radius: 12px; padding: 24px; display: inline-block; margin-bottom: 30px;">
                        <span style="font-family: 'Courier New', monospace; font-size: 36px; font-weight: 700; color: #ffffff; letter-spacing: 8px;">{}</span>
                    </div>
                    
                    <p style="margin: 0; color: #71717a; font-size: 13px;">This code will expire in 15 minutes.</p>
                    <p style="margin: 5px 0 0; color: #71717a; font-size: 13px;">If you didn't request this, you can safely ignore this email.</p>
                </div>
            </div>
            
            <div style="text-align: center; margin-top: 30px; font-size: 12px; color: #52525b;">
                <p style="margin: 0 0 5px; font-weight: 600; color: #71717a;">&copy; 2026 silo.cat</p>
                <p style="margin: 0;">Clickswave Labs Private Limited</p>
            </div>
        </div>
        "#,
        to_name,
        otp
    );

    let email_data = EmailData {
        to_name: to_name.to_string(),
        to_email: to_email.to_string(),
        from_name: smtp_config.from_name.clone(),
        from_email: smtp_config.from_email.clone(),
        reply_to_name: smtp_config.reply_to_name.clone(),
        reply_to_email: smtp_config.reply_to_email.clone(),
        email_subject: "Verify your SiloCat account".to_string(),
        email_body,
    };

    send(smtp_config, &email_data).await
}