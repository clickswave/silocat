UPDATE admin_users 
SET password_hash = '$argon2id$v=19$m=19456,t=2,p=1$REDACTED$REDACTED'
WHERE email = 'admin@silo.cat';
