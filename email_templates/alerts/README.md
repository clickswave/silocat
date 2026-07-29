# Silocat alert emails

Product alerts: mail the **system** emits about something that happened, rather
than mail a user's own action asked for. Everything a user triggers directly
(verify email, password reset, ticket reply, suspension notice) is
transactional and belongs in `../transactional/` instead.

The split is not cosmetic. Each category sends under its own SES custom MAIL
FROM domain, so bounce and complaint reputation is tracked separately:

| Category | Folder | MAIL FROM |
|----------|--------|-----------|
| Transactional | `../transactional/` | `mail.silo.cat` |
| Alerts | this folder | `notify.silo.cat` |

That matters because alerts go to a wider, less engaged audience than password
resets do. If a burst of alerts starts bouncing, the damage is contained to the
alert domain and password resets still reach the inbox.

**This folder is empty today.** Silocat has no product alerts yet; every email
it sends is transactional. The folder exists so the first one has an obvious
home and lands on the right MAIL FROM from day one.

## Adding one

1. Add an entry to `PAGES` in `../build.py` with `category="alerts"`, then run
   `python3 projects/silocat/email_templates/build.py`.
2. Register it in `TEMPLATES` in `scripts/silocat_ses_templates.py` with the
   path `alerts/<key>.html`, then seed it:
   `python3 scripts/silocat_ses_templates.py --only <key>`.
3. Send it from the alert identity, not `team@silo.cat`, or it goes out under
   the transactional MAIL FROM and the separation buys nothing.
