use uuid::Uuid;
use ring;

pub fn generate_sha1_hash() -> String {
    let uuid = Uuid::new_v4();
    ring::digest::digest(&ring::digest::SHA1, uuid.as_bytes())
        .as_ref()
        .into_iter()
        .fold(String::new(), |mut acc, c| {
            acc.push_str(&format!("{:x}", c));
            acc
        })
}

pub fn generate_sha1_random() -> String {
    use ring::rand::SecureRandom;
    let rand = ring::rand::SystemRandom::new();
    let mut buf = vec![0u8; 32];
    rand.fill(&mut buf).unwrap();
    ring::digest::digest(&ring::digest::SHA1, &buf)
        .as_ref()
        .into_iter()
        .fold(String::new(), |mut acc, c| {
            acc.push_str(&format!("{:x}", c));
            acc
        })
}
