use hmac::{Hmac, Mac};
use sha2::Sha256;

const SECRET_KEY: &str = r"~d}$Q7$eIni=V)9\RK/P.RM4;9[7|@/CA}b~OW!3?EV`:<>M7pddUBL5n|0/*Cn";

type HmacSha256 = Hmac<Sha256>;

pub fn generate_signature(path: &str, timestamp: &str, nonce: &str, method: &str, api_key: &str) -> String {
    let raw = format!("{}{}{}{}{}", path, timestamp, nonce, method, api_key);
    let raw_lower = raw.to_lowercase();
    let mut mac = HmacSha256::new_from_slice(SECRET_KEY.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(raw_lower.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn generate_nonce() -> String {
    uuid::Uuid::new_v4().to_string().replace("-", "")
}
