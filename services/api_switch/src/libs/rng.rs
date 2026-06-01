use rand::distributions::Alphanumeric;
use rand::Rng;
use uuid::Uuid;

// UUIDv4
pub fn uuid() -> String { String::from(Uuid::new_v4()) }

// numeric string
pub fn number(length: usize) -> String {
    let min = 10_u64.pow((length - 1) as u32);
    let max = 10_u64.pow(length as u32) - 1;
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max).to_string()
}

// alphanumeric string
pub fn string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}

pub fn license_key() -> String {
    let segment_length = 5;
    format!(
        "{}-{}-{}-{}",
        string(segment_length).to_uppercase(),
        string(segment_length).to_uppercase(),
        string(segment_length).to_uppercase(),
        string(segment_length).to_uppercase(),
    )
}
