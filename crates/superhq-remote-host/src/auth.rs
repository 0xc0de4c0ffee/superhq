//! HMAC-based proof verification for `session.hello`.
//!
//! Challenge-response, not timestamped. The server hands out a
//! fresh 32-byte nonce in `session.challenge`; the client HMACs
//! (host_node_id, device_id, nonce) with its device key; the server
//! verifies the HMAC against the nonce it issued and invalidates the
//! nonce on first use. Wall clocks are not in the protocol.

use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const DOMAIN: &str = "superhq:v1:";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid base64 in proof")]
    BadBase64,
    #[error("hmac mismatch")]
    Mismatch,
    #[error("device key wrong length (need 32 bytes)")]
    BadKeyLen,
    #[error("no pending challenge on this connection; call session.challenge first")]
    NoChallenge,
}

/// Build the proof binding `(host_id, device_id, nonce)` under
/// `device_key`.
pub fn compute_proof(
    device_key: &[u8],
    host_node_id: &str,
    device_id: &str,
    nonce: &[u8],
) -> Result<String, AuthError> {
    if device_key.len() != 32 {
        return Err(AuthError::BadKeyLen);
    }
    let mut mac = HmacSha256::new_from_slice(device_key)
        .map_err(|_| AuthError::BadKeyLen)?;
    mac.update(DOMAIN.as_bytes());
    mac.update(host_node_id.as_bytes());
    mac.update(b":");
    mac.update(device_id.as_bytes());
    mac.update(b":");
    mac.update(nonce);
    let tag = mac.finalize().into_bytes();
    Ok(STANDARD.encode(tag))
}

/// Verify a client-provided proof against the nonce the server issued.
pub fn verify_proof(
    device_key: &[u8],
    host_node_id: &str,
    device_id: &str,
    nonce: &[u8],
    proof_b64: &str,
) -> Result<(), AuthError> {
    let claimed = STANDARD
        .decode(proof_b64.as_bytes())
        .map_err(|_| AuthError::BadBase64)?;
    let mut mac = HmacSha256::new_from_slice(device_key)
        .map_err(|_| AuthError::BadKeyLen)?;
    mac.update(DOMAIN.as_bytes());
    mac.update(host_node_id.as_bytes());
    mac.update(b":");
    mac.update(device_id.as_bytes());
    mac.update(b":");
    mac.update(nonce);
    mac.verify_slice(&claimed).map_err(|_| AuthError::Mismatch)
}

/// Generate a fresh 32-byte challenge.
pub fn generate_challenge() -> [u8; 32] {
    use rand::RngCore;
    let mut n = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut n);
    n
}

/// Generate a random 32-byte device key.
pub fn generate_device_key() -> [u8; 32] {
    use rand::RngCore;
    let mut k = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut k);
    k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_proof() {
        let key = [7u8; 32];
        let host = "host-abc";
        let device = "dev-xyz";
        let nonce = [9u8; 32];
        let proof = compute_proof(&key, host, device, &nonce).unwrap();
        verify_proof(&key, host, device, &nonce, &proof).unwrap();
    }

    #[test]
    fn rejects_wrong_nonce() {
        let proof = compute_proof(&[7u8; 32], "h", "d", &[1u8; 32]).unwrap();
        assert!(matches!(
            verify_proof(&[7u8; 32], "h", "d", &[2u8; 32], &proof),
            Err(AuthError::Mismatch)
        ));
    }

    #[test]
    fn rejects_wrong_key() {
        let proof = compute_proof(&[1u8; 32], "h", "d", &[0u8; 32]).unwrap();
        assert!(matches!(
            verify_proof(&[2u8; 32], "h", "d", &[0u8; 32], &proof),
            Err(AuthError::Mismatch)
        ));
    }

    #[test]
    fn rejects_tampered_transcript() {
        let proof = compute_proof(&[7u8; 32], "h", "d", &[0u8; 32]).unwrap();
        assert!(matches!(
            verify_proof(&[7u8; 32], "h", "d2", &[0u8; 32], &proof),
            Err(AuthError::Mismatch)
        ));
    }
}
