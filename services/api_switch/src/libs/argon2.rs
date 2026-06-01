use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};

pub fn hash(password: String) -> Result<String, ()> {
    let salt = SaltString::generate(&mut OsRng);
    // Argon2 with default params (Argon2id v19)
    // do not change if you do not know what you are doing
    let argon2 = Argon2::default();
    // Hash password to PHC string ($argon2id$v=19$...)
    let hash = argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string();
    Ok(hash)
}

pub fn verify(password: &String, hash: String) -> bool {
    // parse hash from string to usable format
    let parsed_hash = PasswordHash::new(&hash).expect("Exception while trying to hash password");
    // verify password and hash
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        // true if ok
        .is_ok()
}
