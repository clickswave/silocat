//! Accounts the admin console's Growth screen leaves out: ours and the test
//! fixtures. GROWTH_INTERNAL_EMAILS is a comma list; an entry starting with `@`
//! matches a whole domain, one holding `@` matches one address, and anything
//! else matches when the address contains it. Unset, the default list applies.

pub const DEFAULT: &str = "@clickswave.org,@clickswavelabs.com,@silo.cat,@example.com,@example.net,@example.org,@example.invalid";

pub fn patterns() -> Vec<String> {
    let raw = std::env::var("GROWTH_INTERNAL_EMAILS").ok().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| DEFAULT.to_string());
    raw.split(',').map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()).collect()
}

pub fn is_internal(email: &str, patterns: &[String]) -> bool {
    let e = email.trim().to_ascii_lowercase();
    if e.is_empty() {
        return false;
    }
    patterns.iter().any(|p| {
        if p.starts_with('@') {
            e.ends_with(p.as_str())
        } else if p.contains('@') {
            e == *p
        } else {
            e.contains(p.as_str())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching() {
        let p: Vec<String> = ["@clickswave.org", "ops@silo.cat", "qa-bot"].iter().map(|s| s.to_string()).collect();
        assert!(is_internal("Dikshant@ClicksWave.org", &p));
        assert!(!is_internal("a@notclickswave.org.example", &p));
        assert!(is_internal("ops@silo.cat", &p));
        assert!(!is_internal("ops2@silo.cat", &p));
        assert!(is_internal("qa-bot+t1@gmail.com", &p));
        assert!(!is_internal("", &p));
        assert!(!is_internal("someone@gmail.com", &p));
    }
}
