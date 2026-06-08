use lettre::{Message, SmtpTransport, Transport};
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::transport::smtp::response::Response;
use serde::Serialize;
use crate::libs::configs::SmtpConfig;

/// Logo embedded inline (referenced as `cid:silocat-logo` in templates) so it
/// always renders without depending on a hosted asset URL.
const LOGO_PNG: &[u8] = include_bytes!("../../assets/silocat-logo.png");
const LOGO_CID: &str = "silocat-logo";

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
    // construct email: HTML + inline logo (multipart/related, cid:silocat-logo)
    let logo_part = Attachment::new_inline(LOGO_CID.to_string())
        .body(LOGO_PNG.to_vec(), "image/png".parse().expect("invalid logo content type"));
    let email = Message::builder()
        .from(mail_from.parse().expect("Exception while parsing mail_from"))
        .reply_to(mail_reply_to.parse().expect("Exception while parsing mail_reply_to"))
        .to(mail_to.parse().expect("Exception while parsing mail_to"))
        .subject(&email_data.email_subject)
        .multipart(
            MultiPart::related()
                .singlepart(SinglePart::html(email_data.email_body.clone()))
                .singlepart(logo_part),
        )
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

/// Minimal HTML escaping for values interpolated into the email template.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Branded, email-client-safe (table-based, inline-styled) OTP email.
/// Dark theme matching silo.cat, hosted cat logo, preheader text, and a
/// bulletproof CTA button.
fn render_otp_email(
    name: &str,
    heading: &str,
    intro: &str,
    otp: &str,
    cta_label: &str,
    cta_url: &str,
    preheader: &str,
    security_note: &str,
) -> String {
    const TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="en" xmlns="http://www.w3.org/1999/xhtml">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="x-apple-disable-message-reformatting">
<meta http-equiv="X-UA-Compatible" content="IE=edge">
<title>SiloCat</title>
<style>
  @media only screen and (max-width:600px){
    .container{width:100% !important}
    .card{padding:28px 22px !important}
    .code{font-size:30px !important;letter-spacing:8px !important}
  }
  body,table,td,a{-webkit-text-size-adjust:100%;-ms-text-size-adjust:100%}
  a{text-decoration:none}
</style>
</head>
<body style="margin:0;padding:0;background:#0a0a0c;background-color:#0a0a0c;">
  <div style="display:none;max-height:0;overflow:hidden;opacity:0;color:#0a0a0c;font-size:1px;line-height:1px;">{{PREHEADER}}&#8203;&#8203;&#8203;&#8203;&#8203;&#8203;&#8203;&#8203;&#8203;&#8203;</div>
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="background:#0a0a0c;background-color:#0a0a0c;">
    <tr>
      <td align="center" style="padding:36px 16px;">
        <table role="presentation" class="container" width="600" cellpadding="0" cellspacing="0" border="0" style="width:600px;max-width:600px;">
          <tr>
            <td style="padding:4px 6px 22px;">
              <table role="presentation" cellpadding="0" cellspacing="0" border="0"><tr>
                <td style="vertical-align:middle;padding-right:12px;">
                  <img src="cid:silocat-logo" width="42" height="42" alt="SiloCat" style="display:block;border-radius:11px;">
                </td>
                <td style="vertical-align:middle;">
                  <span style="font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:21px;font-weight:800;color:#ffffff;letter-spacing:1.5px;">SILO.CAT</span>
                </td>
              </tr></table>
            </td>
          </tr>
          <tr>
            <td class="card" style="background:#161618;background-color:#161618;border:1px solid #2a2a30;border-radius:16px;padding:40px;">
              <div style="height:3px;width:48px;border-radius:99px;background:#ff4655;background-image:linear-gradient(90deg,#ff4655,#ff8a93);margin-bottom:26px;"></div>
              <h1 style="margin:0 0 18px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:24px;line-height:1.25;font-weight:800;color:#ffffff;">{{HEADING}}</h1>
              <p style="margin:0 0 14px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#e9e9ee;">Hi {{NAME}},</p>
              <p style="margin:0 0 28px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#a8a8b2;">{{INTRO}}</p>
              <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0"><tr>
                <td align="center" style="background:#0e0e10;background-color:#0e0e10;border:1px solid #ff4655;border-radius:12px;padding:24px 12px;">
                  <div style="font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:11px;font-weight:600;letter-spacing:2px;text-transform:uppercase;color:#71717a;margin-bottom:10px;">Your code</div>
                  <div class="code" style="font-family:'Courier New',Courier,monospace;font-size:38px;font-weight:700;color:#ffffff;letter-spacing:12px;text-indent:12px;">{{OTP}}</div>
                </td>
              </tr></table>
              <p style="margin:16px 0 28px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;color:#71717a;text-align:center;">This code expires in 15 minutes.</p>
              <table role="presentation" cellpadding="0" cellspacing="0" border="0" align="center" style="margin:0 auto;"><tr>
                <td align="center" bgcolor="#ff4655" style="border-radius:10px;background:#ff4655;background-image:linear-gradient(90deg,#ff4655,#ff8a93);">
                  <a href="{{CTA_URL}}" target="_blank" style="display:inline-block;padding:13px 30px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;font-weight:700;color:#ffffff;border-radius:10px;">{{CTA_LABEL}}</a>
                </td>
              </tr></table>
              <div style="height:1px;background:#26262c;margin:32px 0 0;line-height:1px;font-size:1px;">&nbsp;</div>
              <p style="margin:22px 0 0;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;line-height:1.6;color:#6b6b75;">{{SECURITY_NOTE}}</p>
            </td>
          </tr>
          <tr>
            <td style="padding:24px 8px;text-align:center;">
              <p style="margin:0 0 10px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;color:#8a8a93;">
                <a href="https://silo.cat" target="_blank" style="color:#9a9aa3;">silo.cat</a> &nbsp;&middot;&nbsp;
                <a href="https://silo.cat/privacy" target="_blank" style="color:#9a9aa3;">Privacy</a> &nbsp;&middot;&nbsp;
                <a href="https://silo.cat/policies/terms-of-service" target="_blank" style="color:#9a9aa3;">Terms</a> &nbsp;&middot;&nbsp;
                <a href="mailto:support@silo.cat" style="color:#9a9aa3;">Support</a>
              </p>
              <p style="margin:0;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;color:#52525b;">&copy; 2026 Clickswave Labs Private Limited. Zero-knowledge by design.</p>
              <p style="margin:6px 0 0;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:11px;color:#3f3f46;">You received this email because this address was used on silo.cat.</p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>"##;

    TEMPLATE
        .replace("{{PREHEADER}}", &esc(preheader))
        .replace("{{HEADING}}", &esc(heading))
        .replace("{{NAME}}", &esc(name))
        .replace("{{INTRO}}", &esc(intro))
        .replace("{{OTP}}", &esc(otp))
        .replace("{{CTA_LABEL}}", &esc(cta_label))
        .replace("{{CTA_URL}}", cta_url)
        .replace("{{SECURITY_NOTE}}", &esc(security_note))
}

pub async fn send_verification_email(
    smtp_config: &SmtpConfig,
    to_name: &str,
    to_email: &str,
    otp: &str,
) -> anyhow::Result<Response, String> {
    let email_body = render_otp_email(
        to_name,
        "Confirm your email",
        "Welcome to SiloCat. Enter the code below to verify your email and unlock your encrypted vault.",
        otp,
        "Open SiloCat",
        "https://silo.cat/auth/signin",
        "Your SiloCat verification code is inside. It expires in 15 minutes.",
        "If you didn't create a SiloCat account, you can safely ignore this email.",
    );

    let email_data = EmailData {
        to_name: to_name.to_string(),
        to_email: to_email.to_string(),
        from_name: smtp_config.from_name.clone(),
        from_email: smtp_config.from_email.clone(),
        reply_to_name: smtp_config.reply_to_name.clone(),
        reply_to_email: smtp_config.reply_to_email.clone(),
        email_subject: "Your SiloCat verification code".to_string(),
        email_body,
    };

    send(smtp_config, &email_data).await
}

pub async fn send_password_reset_email(
    smtp_config: &SmtpConfig,
    to_name: &str,
    to_email: &str,
    otp: &str,
) -> anyhow::Result<Response, String> {
    let email_body = render_otp_email(
        to_name,
        "Reset your password",
        "We received a request to reset your SiloCat password. Enter the code below on the sign-in screen to choose a new one.",
        otp,
        "Reset password",
        "https://silo.cat/auth/signin",
        "Your SiloCat password reset code is inside. It expires in 15 minutes.",
        "If you didn't request this, you can safely ignore this email and your password will stay the same.",
    );

    let email_data = EmailData {
        to_name: to_name.to_string(),
        to_email: to_email.to_string(),
        from_name: smtp_config.from_name.clone(),
        from_email: smtp_config.from_email.clone(),
        reply_to_name: smtp_config.reply_to_name.clone(),
        reply_to_email: smtp_config.reply_to_email.clone(),
        email_subject: "Reset your SiloCat password".to_string(),
        email_body,
    };

    send(smtp_config, &email_data).await
}