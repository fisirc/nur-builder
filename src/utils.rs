use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn verify_signature(signature: &str, body: &[u8], secret: &str) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    let expected = format!("sha256={:x}", mac.finalize().into_bytes());
    signature == expected
}