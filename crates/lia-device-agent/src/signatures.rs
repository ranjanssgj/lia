//! Domain-separated Ed25519/JCS primitives for P4 signed messages.
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug)]
pub enum SignatureError {
    Canonicalization(String),
    InvalidPublicKey,
    InvalidSignature,
}
impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonicalization(e) => write!(f, "JCS canonicalization failed: {e}"),
            Self::InvalidPublicKey => write!(f, "invalid Ed25519 public key"),
            Self::InvalidSignature => write!(f, "signature verification failed"),
        }
    }
}
impl std::error::Error for SignatureError {}

pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, SignatureError> {
    serde_jcs::to_vec(value).map_err(|e| SignatureError::Canonicalization(e.to_string()))
}
pub fn command_signing_input(canonical_payload: &[u8]) -> Vec<u8> {
    [b"LIA-COMMAND-V1\n".as_slice(), canonical_payload].concat()
}
pub fn lease_signing_input(canonical_payload: &[u8]) -> Vec<u8> {
    [b"LIA-OFFLINE-LEASE-V1\n".as_slice(), canonical_payload].concat()
}
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub fn verify_command(
    public_key: &[u8; 32],
    signature: &[u8; 64],
    canonical_payload: &[u8],
) -> Result<(), SignatureError> {
    verify(
        public_key,
        signature,
        &command_signing_input(canonical_payload),
    )
}
pub fn verify_lease(
    public_key: &[u8; 32],
    signature: &[u8; 64],
    canonical_payload: &[u8],
) -> Result<(), SignatureError> {
    verify(
        public_key,
        signature,
        &lease_signing_input(canonical_payload),
    )
}
fn verify(
    public_key: &[u8; 32],
    signature: &[u8; 64],
    message: &[u8],
) -> Result<(), SignatureError> {
    let key = VerifyingKey::from_bytes(public_key).map_err(|_| SignatureError::InvalidPublicKey)?;
    key.verify_strict(message, &Signature::from_bytes(signature))
        .map_err(|_| SignatureError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn rfc8032_test_vector_1_and_domain_separation() {
        let vector: serde_json::Value = serde_json::from_str(include_str!(
            "../../../testdata/security/ed25519-rfc8032.json"
        ))
        .unwrap();
        let public: [u8; 32] = hex(vector["publicKeyHex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let signature_bytes: [u8; 64] = hex(vector["signatureHex"].as_str().unwrap())
            .try_into()
            .unwrap();
        let message = hex(vector["messageHex"].as_str().unwrap());
        let public_key = VerifyingKey::from_bytes(&public).unwrap();
        public_key
            .verify_strict(&message, &Signature::from_bytes(&signature_bytes))
            .unwrap();
        let seed: [u8; 32] =
            hex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .try_into()
                .unwrap();
        let signing = SigningKey::from_bytes(&seed);
        let signature = signing.sign(b"");
        assert_eq!(
            to_hex(&signing.verifying_key().to_bytes()),
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        );
        assert_eq!(to_hex(&signature.to_bytes()), "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155 5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b".replace(' ', ""));
        signing
            .verifying_key()
            .verify_strict(b"", &signature)
            .unwrap();
        let command = signing.sign(&command_signing_input(b"{}"));
        verify_command(
            &signing.verifying_key().to_bytes(),
            &command.to_bytes(),
            b"{}",
        )
        .unwrap();
        assert!(
            verify_lease(
                &signing.verifying_key().to_bytes(),
                &command.to_bytes(),
                b"{}"
            )
            .is_err()
        );
    }
    #[test]
    fn canonicalizes_json_and_hashes_bytes() {
        let vector: serde_json::Value =
            serde_json::from_str(include_str!("../../../testdata/security/jcs-sha256.json"))
                .unwrap();
        let value: serde_json::Value =
            serde_json::from_str(vector["inputJson"].as_str().unwrap()).unwrap();
        let bytes = canonical_json(&value).unwrap();
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            vector["canonicalJson"].as_str().unwrap()
        );
        assert_eq!(
            to_hex(&sha256(&bytes)),
            vector["sha256Hex"].as_str().unwrap()
        );
    }
    #[test]
    fn jcs_number_format_and_utf16_key_order_vectors() {
        let numbers: serde_json::Value = serde_json::from_str(
            r#"{"numbers":[333333333.33333329,1E30,4.50,2e-3,0.000000000000000000000000001]}"#,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(canonical_json(&numbers).unwrap()).unwrap(),
            r#"{"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27]}"#
        );
        let keys: serde_json::Value =
            serde_json::from_str(r#"{"\uE000":1,"\uD83D\uDE00":2}"#).unwrap();
        assert_eq!(
            String::from_utf8(canonical_json(&keys).unwrap()).unwrap(),
            r#"{"😀":2,"":1}"#
        );
    }
    #[test]
    fn signature_rejects_wrong_payload_key_domain_truncation_and_noncanonical_s() {
        use ed25519_dalek::{Signer, SigningKey};
        let signing = SigningKey::from_bytes(&[7; 32]);
        let payload = br#"{"x":1}"#;
        let sig = signing.sign(&command_signing_input(payload)).to_bytes();
        assert!(verify_command(&signing.verifying_key().to_bytes(), &sig, payload).is_ok());
        assert!(verify_command(&signing.verifying_key().to_bytes(), &sig, br#"{"x":2}"#).is_err());
        assert!(verify_lease(&signing.verifying_key().to_bytes(), &sig, payload).is_err());
        let other = SigningKey::from_bytes(&[8; 32]);
        assert!(verify_command(&other.verifying_key().to_bytes(), &sig, payload).is_err());
        assert!(<[u8; 64]>::try_from(&sig[..63]).is_err());
        let mut noncanonical = sig;
        noncanonical[63] |= 0x80;
        assert!(
            verify_command(&signing.verifying_key().to_bytes(), &noncanonical, payload).is_err()
        );
    }
    fn hex(s: &str) -> Vec<u8> {
        let chars: Vec<_> = s.chars().filter(|c| !c.is_whitespace()).collect();
        chars
            .chunks(2)
            .map(|p| u8::from_str_radix(&p.iter().collect::<String>(), 16).unwrap())
            .collect()
    }
    fn to_hex(b: &[u8]) -> String {
        b.iter().map(|v| format!("{v:02x}")).collect()
    }
}
