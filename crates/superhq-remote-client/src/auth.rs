//! Client-side HMAC proof generation for `session.hello`.
//!
//! Mirrors the host's `verify_proof`: HMAC-SHA256 over
//!   "superhq:v1:" || host_node_id || ":" || device_id || ":" || nonce_bytes
//! keyed by the device key. The nonce is the 32 raw bytes the server
//! just handed out via `session.challenge`.

use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const DOMAIN: &str = "superhq:v1:";

pub fn compute_proof(
    device_key: &[u8],
    host_node_id: &str,
    device_id: &str,
    nonce: &[u8],
) -> Result<String, &'static str> {
    if device_key.len() != 32 {
        return Err("device key must be 32 bytes");
    }
    let mut mac = HmacSha256::new_from_slice(device_key).map_err(|_| "hmac init")?;
    mac.update(DOMAIN.as_bytes());
    mac.update(host_node_id.as_bytes());
    mac.update(b":");
    mac.update(device_id.as_bytes());
    mac.update(b":");
    mac.update(nonce);
    let tag = mac.finalize().into_bytes();
    Ok(STANDARD.encode(tag))
}

pub fn decode_device_key(b64: &str) -> Result<Vec<u8>, &'static str> {
    STANDARD
        .decode(b64.as_bytes())
        .map_err(|_| "invalid base64 device key")
}

pub fn decode_nonce(b64: &str) -> Result<Vec<u8>, &'static str> {
    STANDARD
        .decode(b64.as_bytes())
        .map_err(|_| "invalid base64 nonce")
}
