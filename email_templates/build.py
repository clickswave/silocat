#!/usr/bin/env python3
"""Generate the Silocat transactional emails from one shared shell.

The header, footer, palette and type live here once. Each template supplies
only its own body, so the chrome cannot drift between emails the way it does
when every file is a copy of the last one.

Run after editing: python3 projects/silocat/email_templates/build.py
"""
import base64, pathlib, subprocess, tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
# Emails are filed by category, because each category sends under its own SES
# MAIL FROM domain: "transactional" (mail the user's own action triggered) under
# mail.silo.cat, "alerts" (product alerts the system emits) under notify.silo.cat.
# A page picks its folder with category="alerts"; transactional is the default.
OUT_ROOT = ROOT / "email_templates"
DEFAULT_CATEGORY = "transactional"
LOGO_SRC = ROOT / "services" / "web_server" / "static" / "silocat-logo.png"

# Ink & Signal, straight from global.scss. Flat, hairline, one accent.
BG, CARD, EDGE = "#0b0b0d", "#111114", "#26262b"
INK, DIM, FAINT, ACCENT = "#f2f2f4", "#a0a0a8", "#61616b", "#ff4655"
SANS = "'Inter','Helvetica Neue',Arial,sans-serif"
MONO = "'JetBrains Mono','SF Mono',Menlo,monospace"


def logo_b64() -> str:
    """452px square is far too heavy to inline; 128 keeps it under 20KB."""
    with tempfile.NamedTemporaryFile(suffix=".png") as t:
        for exe in ("convert", "magick"):
            try:
                subprocess.run([exe, str(LOGO_SRC), "-resize", "128x128",
                                "-strip", t.name], check=True,
                               capture_output=True)
                break
            except (FileNotFoundError, subprocess.CalledProcessError):
                continue
        else:
            raise SystemExit("need ImageMagick (convert/magick) to size the logo")
        return base64.b64encode(pathlib.Path(t.name).read_bytes()).decode()


def shell(title: str, preheader: str, body: str, logo: str) -> str:
    return f"""<!doctype html>
<html lang="en" style="margin:0;padding:0;">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="x-apple-disable-message-reformatting">
  <meta name="color-scheme" content="dark light">
  <meta name="supported-color-schemes" content="dark light">
  <title>{title}</title>
  <style type="text/css">
    @media only screen and (max-width:600px){{
      .sc-outer {{ padding:28px 10px !important; }}
      .sc-px    {{ padding-left:22px !important; padding-right:22px !important; }}
      .sc-code  {{ font-size:30px !important; letter-spacing:8px !important; text-indent:8px !important; }}
    }}
  </style>
</head>
<body style="margin:0;padding:0;background-color:{BG};color-scheme:dark light;">
  <div style="display:none;max-height:0;overflow:hidden;mso-hide:all;opacity:0;color:transparent;height:0;width:0;">{preheader}&nbsp;&zwnj;&nbsp;&zwnj;&nbsp;&zwnj;&nbsp;&zwnj;&nbsp;&zwnj;&nbsp;&zwnj;</div>

  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="background-color:{BG};">
    <tr>
      <td align="center" class="sc-outer" style="padding:52px 16px;">
        <table role="presentation" width="512" cellpadding="0" cellspacing="0" border="0" style="width:100%;max-width:512px;background-color:{CARD};border:1px solid {EDGE};">

          <tr><td style="height:2px;line-height:2px;font-size:2px;background-color:{ACCENT};">&nbsp;</td></tr>

          <tr>
            <td align="center" class="sc-px" style="padding:34px 48px 24px;font-family:{SANS};">
              <img src="data:image/png;base64,{logo}" alt="" width="52" style="display:block;margin:0 auto 12px;width:52px;max-width:52px;height:auto;border:0;" />
              <div style="font-size:17px;font-weight:600;letter-spacing:-0.01em;color:{INK};">silocat</div>
            </td>
          </tr>

          <tr><td style="height:1px;line-height:1px;font-size:1px;background-color:{EDGE};">&nbsp;</td></tr>
{body}
          <tr><td style="height:1px;line-height:1px;font-size:1px;background-color:{EDGE};">&nbsp;</td></tr>

          <tr>
            <td align="center" class="sc-px" style="padding:22px 48px 26px;font-family:{SANS};font-size:12px;line-height:18px;color:{FAINT};">
              <a href="https://silo.cat" style="color:{FAINT};text-decoration:none;">silo.cat</a>
              &nbsp;&middot;&nbsp; end-to-end encrypted file sharing
            </td>
          </tr>

        </table>
      </td>
    </tr>
  </table>
</body>
</html>
"""


def heading(eyebrow: str, title: str, lede: str) -> str:
    return f"""
          <tr>
            <td align="center" class="sc-px" style="padding:34px 48px 0;font-family:{SANS};">
              <div style="font-size:11px;font-weight:600;letter-spacing:2.5px;text-transform:uppercase;color:{ACCENT};">{eyebrow}</div>
              <div style="padding-top:12px;font-size:22px;font-weight:600;letter-spacing:-0.02em;color:{INK};">{title}</div>
              <div style="padding-top:14px;font-size:14px;line-height:22px;color:{DIM};">{lede}</div>
            </td>
          </tr>
"""


def code_block(expiry: str) -> str:
    return f"""
          <tr>
            <td align="center" class="sc-px" style="padding:28px 48px 0;">
              <table role="presentation" cellpadding="0" cellspacing="0" border="0" style="border:1px solid {EDGE};background-color:{BG};">
                <tr>
                  <td align="center" class="sc-code" style="padding:18px 30px;font-family:{MONO};font-size:34px;font-weight:600;letter-spacing:11px;text-indent:11px;color:{INK};">{{{{CODE}}}}</td>
                </tr>
              </table>
              <div style="padding-top:12px;font-size:12px;line-height:18px;color:{FAINT};font-family:{SANS};">{expiry}</div>
            </td>
          </tr>
"""


def callout(text: str) -> str:
    return f"""
          <tr>
            <td class="sc-px" style="padding:28px 48px 0;font-family:{SANS};">
              <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="border-left:2px solid {ACCENT};background-color:#0e0e11;">
                <tr><td style="padding:14px 16px;font-size:13px;line-height:20px;color:{DIM};">{text}</td></tr>
              </table>
            </td>
          </tr>
"""


def button(label: str, href: str) -> str:
    return f"""
          <tr>
            <td align="center" class="sc-px" style="padding:28px 48px 0;font-family:{SANS};">
              <table role="presentation" cellpadding="0" cellspacing="0" border="0">
                <tr><td align="center" style="background-color:{ACCENT};">
                  <a href="{href}" style="display:inline-block;padding:12px 26px;font-size:14px;font-weight:600;color:#ffffff;text-decoration:none;">{label}</a>
                </td></tr>
              </table>
            </td>
          </tr>
"""


def note(text: str) -> str:
    return f"""
          <tr>
            <td class="sc-px" style="padding:24px 48px 34px;font-family:{SANS};font-size:13px;line-height:20px;color:{FAINT};">{text}</td>
          </tr>
"""


ZK = ("Your files stay encrypted throughout. We hold ciphertext and never see "
      "the passwords that unlock it, so nothing here gives us or anyone else "
      "access to their contents.")

PAGES = {
    "password-reset": dict(
        title="Reset your Silocat password",
        preheader="Reset your Silocat password. This code expires in 15 minutes.",
        body=lambda: (
            heading("Account security", "Reset your password",
                    "Hi {{NAME}}, enter this code to set a new password for your Silocat account.")
            + code_block("Expires in 15 minutes.")
            + callout("This resets your <strong style='color:%s;font-weight:600;'>account</strong> password only. "
                      "It does not unlock your encrypted files: those are protected by the passwords you chose "
                      "when uploading, which we never receive and cannot recover." % INK)
            + note("Didn't request this? Ignore this email and your password stays as it is.")
        ),
    ),
    "verify-email": dict(
        title="Verify your email - Silocat",
        preheader="Enter this code to verify your email and activate your Silocat account.",
        body=lambda: (
            heading("Welcome", "Verify your email",
                    "Hi {{NAME}}, enter this code to confirm your address and activate your Silocat account.")
            + code_block("Expires in 15 minutes.")
            + callout(ZK)
            + note("Didn't create a Silocat account? Ignore this email and nothing further happens.")
        ),
    ),
    "ticket-update": dict(
        title="Update on your Silocat ticket",
        preheader="There's a reply on your Silocat support ticket.",
        body=lambda: (
            heading("Support", "There's a reply on your ticket",
                    "Hi {{NAME}}, someone has responded to the ticket you opened.")
            + f"""
          <tr>
            <td class="sc-px" style="padding:26px 48px 0;font-family:{SANS};">
              <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="border:1px solid {EDGE};background-color:{BG};">
                <tr><td style="padding:16px 18px;">
                  <div style="font-family:{MONO};font-size:12px;color:{FAINT};">{{{{TICKET_ID}}}}</div>
                  <div style="padding-top:6px;font-size:15px;font-weight:600;color:{INK};">{{{{TICKET_SUBJECT}}}}</div>
                  <div style="padding-top:10px;font-size:13px;line-height:20px;color:{DIM};">{{{{MESSAGE}}}}</div>
                </td></tr>
              </table>
            </td>
          </tr>
"""
            + button("View the ticket", "https://silo.cat/home/support/{{TICKET_ID}}")
            + note("Reply from the ticket page so the whole thread stays in one place.")
        ),
    ),
    "account-suspended": dict(
        title="Your Silocat account has been suspended",
        preheader="Your Silocat account has been suspended. Your files are untouched.",
        body=lambda: (
            heading("Account status", "Your account is suspended",
                    "Hi {{NAME}}, access to your Silocat account ({{EMAIL}}) has been suspended.")
            + f"""
          <tr>
            <td class="sc-px" style="padding:26px 48px 0;font-family:{SANS};">
              <table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="border:1px solid {EDGE};background-color:{BG};">
                <tr><td style="padding:16px 18px;font-size:13px;line-height:20px;color:{DIM};">
                  <div style="font-size:11px;font-weight:600;letter-spacing:2px;text-transform:uppercase;color:{FAINT};">Reason</div>
                  <div style="padding-top:8px;color:{INK};">{{{{REASON}}}}</div>
                </td></tr>
              </table>
            </td>
          </tr>
"""
            + callout("Your files have not been deleted and remain encrypted exactly as you left them. "
                      "Suspension removes access, not data.")
            + button("Appeal this decision", "https://silo.cat/home/support")
            + note("If you think this is a mistake, open an appeal and a human will read it.")
        ),
    ),
}


def main() -> None:
    logo = logo_b64()
    for key, spec in PAGES.items():
        out_dir = OUT_ROOT / spec.get("category", DEFAULT_CATEGORY)
        out_dir.mkdir(parents=True, exist_ok=True)
        html = shell(spec["title"], spec["preheader"], spec["body"](), logo)
        p = out_dir / f"{key}.html"
        p.write_text(html)
        print(f"  {p.relative_to(OUT_ROOT)}  ({len(html):,} bytes)")


if __name__ == "__main__":
    main()
