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

    let mail_to = format!("{} <{}>", &email_data.to_name, &email_data.to_email);
    let mail_from = format!("{} <{}>", &email_data.from_name, &email_data.from_email);
    let mail_reply_to = format!("{} <{}>", &email_data.reply_to_name, &email_data.reply_to_email);
    // construct email: HTML + inline logo (multipart/related, cid:silocat-logo)
    // Never panic on address/content parsing — a bad address returns an error
    // the caller handles (log + continue), it doesn't take the worker down.
    let logo_ct = "image/png".parse().map_err(|e| format!("content type: {e}"))?;
    let logo_part = Attachment::new_inline(LOGO_CID.to_string())
        .body(LOGO_PNG.to_vec(), logo_ct);
    let email = Message::builder()
        .from(mail_from.parse().map_err(|e| format!("from address: {e}"))?)
        .reply_to(mail_reply_to.parse().map_err(|e| format!("reply-to address: {e}"))?)
        .to(mail_to.parse().map_err(|e| format!("to address: {e}"))?)
        .subject(&email_data.email_subject)
        .multipart(
            MultiPart::related()
                .singlepart(SinglePart::html(email_data.email_body.clone()))
                .singlepart(logo_part),
        )
        .map_err(|e| format!("build email: {e}"))?;

    // parse smtp creds
    let creds = Credentials::new(smtp_config.username.clone(), smtp_config.password.clone());
    // parse tls config
    let tls_config = match TlsParameters::builder(smtp_config.address.clone()).build() {
        Ok(tls_config) => tls_config,
        Err(err) => {
            return Err(err.to_string());
        }
    };

    // create mailer
    let mailer = SmtpTransport::relay(
        smtp_config.address.as_str()
    )
        .map_err(|e| e.to_string())?
        .port(587)
        .tls(Tls::Required(tls_config))
        .credentials(creds)
        .build();

    // send email
    match mailer.send(&email) {
        Ok(response) => {
            Ok(response)
        }
        Err(err) => {
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
                <a href="mailto:team@silo.cat" style="color:#9a9aa3;">Support</a>
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

/// Internal support/contact message from a signed-in user. Sent to the support
/// inbox with reply-to set to the user so the team can reply directly.
pub async fn send_support_email(
    smtp_config: &SmtpConfig,
    user_name: &str,
    user_email: &str,
    category: &str,
    subject: &str,
    message: &str,
) -> anyhow::Result<Response, String> {
    let body = format!(
        r##"<!DOCTYPE html><html><body style="margin:0;padding:0;background:#0a0a0c;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="background:#0a0a0c;">
<tr><td align="center" style="padding:32px 16px;">
<table role="presentation" width="600" cellpadding="0" cellspacing="0" border="0" style="width:600px;max-width:600px;">
<tr><td style="padding:0 6px 18px;">
<span style="font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:18px;font-weight:800;color:#ffffff;letter-spacing:1.2px;">SILO.CAT</span>
<span style="font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;color:#71717a;"> &nbsp;support</span>
</td></tr>
<tr><td style="background:#161618;border:1px solid #2a2a30;border-radius:14px;padding:32px;">
<div style="height:3px;width:44px;border-radius:99px;background:#ff4655;margin-bottom:22px;"></div>
<h1 style="margin:0 0 20px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:20px;font-weight:800;color:#fff;">New support message</h1>
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:14px;color:#e9e9ee;">
<tr><td style="padding:6px 0;color:#71717a;width:90px;">Category</td><td style="padding:6px 0;color:#fff;font-weight:600;">{category}</td></tr>
<tr><td style="padding:6px 0;color:#71717a;">From</td><td style="padding:6px 0;color:#fff;">{user_name} &lt;{user_email}&gt;</td></tr>
<tr><td style="padding:6px 0;color:#71717a;">Subject</td><td style="padding:6px 0;color:#fff;font-weight:600;">{subject}</td></tr>
</table>
<div style="height:1px;background:#26262c;margin:20px 0;line-height:1px;font-size:1px;">&nbsp;</div>
<p style="margin:0;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:14px;line-height:1.65;color:#d8d8de;white-space:pre-wrap;">{message}</p>
</td></tr>
<tr><td style="padding:18px 8px;text-align:center;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;color:#52525b;">
Reply directly to this email to respond to {user_name}.
</td></tr>
</table></td></tr></table></body></html>"##,
        category = esc(category),
        user_name = esc(user_name),
        user_email = esc(user_email),
        subject = esc(subject),
        message = esc(message),
    );

    let email_data = EmailData {
        to_name: "SiloCat Support".to_string(),
        to_email: "team@silo.cat".to_string(),
        from_name: smtp_config.from_name.clone(),
        from_email: smtp_config.from_email.clone(),
        // Reply-to is the user so the team can answer them directly.
        reply_to_name: user_name.to_string(),
        reply_to_email: user_email.to_string(),
        email_subject: format!("[Support · {}] {}", category, subject),
        email_body: body,
    };

    send(smtp_config, &email_data).await
}

/// Notify the ticket owner that an admin replied or resolved their ticket.
/// `kind` is "reply" or "resolved". `excerpt` is the admin's reply text (reply only).
pub async fn send_ticket_update_email(
    smtp_config: &SmtpConfig,
    to_name: &str,
    to_email: &str,
    ticket_id: &str,
    ticket_subject: &str,
    kind: &str,
    excerpt: &str,
) -> anyhow::Result<Response, String> {
    let resolved = kind == "resolved";
    let heading = if resolved { "Your ticket was resolved" } else { "New reply from SiloCat" };
    let intro = if resolved {
        format!(
            "We've marked your support ticket \u{201c}{}\u{201d} as resolved. If you still need help, reply on the ticket to reopen it.",
            ticket_subject
        )
    } else {
        format!("The SiloCat team replied to your ticket \u{201c}{}\u{201d}.", ticket_subject)
    };
    let cta_url = format!("https://silo.cat/home/support/{}", ticket_id);

    let excerpt_block = if !resolved && !excerpt.trim().is_empty() {
        format!(
            r##"<div style="background:#0e0e10;border:1px solid #2a2a30;border-radius:10px;padding:16px 18px;margin:0 0 26px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:14px;line-height:1.6;color:#d8d8de;white-space:pre-wrap;">{}</div>"##,
            esc(excerpt)
        )
    } else {
        String::new()
    };

    let body = format!(
        r##"<!DOCTYPE html><html><body style="margin:0;padding:0;background:#0a0a0c;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="background:#0a0a0c;">
<tr><td align="center" style="padding:34px 16px;">
<table role="presentation" width="600" cellpadding="0" cellspacing="0" border="0" style="width:600px;max-width:600px;">
<tr><td style="padding:0 6px 20px;">
<table role="presentation" cellpadding="0" cellspacing="0" border="0"><tr>
<td style="vertical-align:middle;padding-right:11px;"><img src="cid:silocat-logo" width="38" height="38" alt="SiloCat" style="display:block;border-radius:10px;"></td>
<td style="vertical-align:middle;"><span style="font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:19px;font-weight:800;color:#fff;letter-spacing:1.3px;">SILO.CAT</span></td>
</tr></table>
</td></tr>
<tr><td style="background:#161618;border:1px solid #2a2a30;border-radius:16px;padding:38px;">
<div style="height:3px;width:46px;border-radius:99px;background:#ff4655;margin-bottom:24px;"></div>
<h1 style="margin:0 0 16px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:22px;font-weight:800;color:#fff;">{heading}</h1>
<p style="margin:0 0 14px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#e9e9ee;">Hi {name},</p>
<p style="margin:0 0 24px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#a8a8b2;">{intro}</p>
{excerpt_block}
<table role="presentation" cellpadding="0" cellspacing="0" border="0" align="left"><tr>
<td align="center" bgcolor="#ff4655" style="border-radius:10px;background:#ff4655;background-image:linear-gradient(90deg,#ff4655,#ff8a93);">
<a href="{cta_url}" target="_blank" style="display:inline-block;padding:12px 28px;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;font-weight:700;color:#fff;border-radius:10px;">View ticket</a>
</td></tr></table>
</td></tr>
<tr><td style="padding:22px 8px;text-align:center;font-family:'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;color:#52525b;">
&copy; 2026 Clickswave Labs Private Limited &middot; <a href="https://silo.cat/home/support" style="color:#9a9aa3;">Your tickets</a>
</td></tr>
</table></td></tr></table></body></html>"##,
        heading = esc(heading),
        name = esc(to_name),
        intro = esc(&intro),
        excerpt_block = excerpt_block,
        cta_url = cta_url,
    );

    let email_data = EmailData {
        to_name: to_name.to_string(),
        to_email: to_email.to_string(),
        from_name: smtp_config.from_name.clone(),
        from_email: smtp_config.from_email.clone(),
        reply_to_name: smtp_config.reply_to_name.clone(),
        reply_to_email: smtp_config.reply_to_email.clone(),
        email_subject: if resolved {
            format!("Resolved: {}", ticket_subject)
        } else {
            format!("New reply: {}", ticket_subject)
        },
        email_body: body,
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