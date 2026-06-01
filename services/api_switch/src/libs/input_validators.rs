use std::str::FromStr;
use email_address::EmailAddress;
use regex::Regex;

pub fn email(email: &str) -> Result<(), Vec<String>> {
    let mut errors = vec![];

    if email.len() == 0 {
        errors.push("Email cannot be left blank".to_string());
        return Err(errors);
    }

    // if email is not RFC compliant
    if !EmailAddress::is_valid(email) {
        errors.push("Email entered is invalid".to_string());
        return Err(errors);
    }

    // blacklist domains or emails here
    let blacklisted_domains = vec![
        "inboxkitten.com", "dispostable.com", "fuckyou.com",
    ];
    for domain in blacklisted_domains {
        if email.contains(&domain) {
            errors.push("Your email is blacklisted, please use a different email".to_string());
            return Err(errors);
        }
    }

    Ok(())
}
pub fn password(password: &str) -> Result<(), Vec<String>> {
    let mut errors = vec![];


    if password.len() == 0 {
        errors.push("Password cannot be left blank".to_string());
        return Err(errors);
    }


    // password length can not be less than 8
    if password.len() < 8 { errors.push(String::from("Password is too short, minimum length is 8")); }

    // password length can not be more than 47
    if password.len() > 47 { errors.push(String::from("Password is too long, maximum length is 47")) }

    // password cannot contain whitespaces
    if password.chars().any(|c| c.is_whitespace()) { errors.push(String::from("Password cannot contain spaces")) }

    // password should contain at least one lowercase character
    if !password.chars().any(|c| c.is_ascii_lowercase()) { errors.push(String::from("Password must contain a lowercase alphabet")) }

    // password should contain at least one uppercase character
    if !password.chars().any(|c| c.is_ascii_uppercase()) { errors.push(String::from("Password must contain an uppercase alphabet")) }

    // password should contain at least one number
    if !password.chars().any(|c| c.is_numeric()) { errors.push(String::from("Password must contain a number")) }

    if errors.len() != 0 { return Err(errors); }

    Ok(())
}

pub fn name(name: &str) -> Result<(), Vec<String>> {
    let mut errors = vec![];

    if name.len() == 0 {
        errors.push("Name cannot be left blank".to_string());
        return Err(errors);
    }

    // password length can not be less than 8
    if name.len() < 2 { errors.push(String::from("Name is too short, minimum length is 2")) }

    // password length can not be more than 47
    if name.len() > 30 { errors.push(String::from("Name is too long, maximum length is 30")) }

    if !Regex::new(r"^[a-zA-Z\s]+$").unwrap().is_match(name) {
        errors.push(String::from("Name can only contain alphabets and spaces"))
    }

    if errors.len() != 0 { return Err(errors); }

    Ok(())
}


pub fn mobile_number(number: &str) -> Result<(), &str> {

    // password length can not be less than 8
    if number.len() != 10 { return Err("Contact number is invalid"); }

    // password should contain at least one uppercase character
    if number.chars().any(|c| c.is_ascii_alphabetic()) { return Err("Contact number is invalid"); }

    Ok(())
}

pub fn invite_code_is_valid(invite_code: &String) -> Result<(), &str> {
    return if invite_code.len() == 0 || invite_code.len() == 16 { Ok(()) } else { Err("Invite code is invalid") };
}

pub fn uuid(uuid: &str) -> Result<(), Vec<String>> {
    // check if uuid is a valid uuid
    let valid_uuid = uuid::Uuid::from_str(uuid);
    return match valid_uuid {
        Ok(_) => Ok(()),
        Err(_) => Err(vec!["Uuid entered is invalid".to_string()]),
    };
}