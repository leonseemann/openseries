//! Ed25519 signatures of the release packages (ADR 0006 §3).
//!
//! The signed message binds the package file name (it contains the version) and the SHA-256 of
//! its bytes, so a package can neither be altered nor passed off as another version. Keys and
//! signatures travel as standard base64 text.

use std::fmt::Write as _;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use sha2::{Digest as _, Sha256};

/// Public half of the release signing key (the private half is the `UPDATE_SIGNING_KEY`
/// repository secret; `docs/architecture/release.md`).
pub const PUBLIC_KEY: &str = "ClE8RAK2TY+R69CUvAHBAEVGemHhmftkq440OVo+duk=";

/// Domain separation, so a signature cannot be reused for another purpose.
const CONTEXT: &str = "openseries-update-v1";

/// Why a package was rejected.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SignatureError {
    /// The key text is not a base64 Ed25519 key.
    #[error("invalid signing key")]
    Key,
    /// The signature file is not a base64 Ed25519 signature.
    #[error("the signature file is damaged")]
    Malformed,
    /// The signature does not match: the package was altered or not built by our CI.
    #[error("the package signature does not match; the download was not installed")]
    Mismatch,
}

/// The exact bytes that are signed for package `name` with content `bytes`.
#[must_use]
pub fn signed_message(name: &str, bytes: &[u8]) -> Vec<u8> {
    let digest = Sha256::digest(bytes);
    let hex = digest.iter().fold(String::new(), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    });
    format!("{CONTEXT}\n{name}\n{hex}").into_bytes()
}

/// Checks `signature` (the `.sig` file text) of package `name` against the built-in key.
///
/// # Errors
/// If the signature is malformed or does not match.
pub fn verify(name: &str, bytes: &[u8], signature: &str) -> Result<(), SignatureError> {
    verify_with(PUBLIC_KEY, name, bytes, signature)
}

/// Like [`verify`] with an explicit base64 public key.
///
/// # Errors
/// If the key or signature is malformed or the signature does not match.
pub fn verify_with(
    public_key: &str,
    name: &str,
    bytes: &[u8],
    signature: &str,
) -> Result<(), SignatureError> {
    let key = decode::<32>(public_key).ok_or(SignatureError::Key)?;
    let key = VerifyingKey::from_bytes(&key).map_err(|_| SignatureError::Key)?;
    let signature = decode::<64>(signature).ok_or(SignatureError::Malformed)?;
    key.verify(
        &signed_message(name, bytes),
        &Signature::from_bytes(&signature),
    )
    .map_err(|_| SignatureError::Mismatch)
}

/// Signs package `name` with the base64 private key `secret`; returns the `.sig` file text.
///
/// # Errors
/// If `secret` is not a base64 32-byte Ed25519 seed.
pub fn sign(secret: &str, name: &str, bytes: &[u8]) -> Result<String, SignatureError> {
    let seed = decode::<32>(secret).ok_or(SignatureError::Key)?;
    let signature = SigningKey::from_bytes(&seed).sign(&signed_message(name, bytes));
    Ok(BASE64.encode(signature.to_bytes()))
}

/// The base64 public key belonging to the base64 private key `secret`.
///
/// # Errors
/// If `secret` is not a base64 32-byte Ed25519 seed.
pub fn public_key_of(secret: &str) -> Result<String, SignatureError> {
    let seed = decode::<32>(secret).ok_or(SignatureError::Key)?;
    Ok(BASE64.encode(SigningKey::from_bytes(&seed).verifying_key().to_bytes()))
}

/// A new random private key (base64), for `ss-release-sign keygen`.
///
/// # Errors
/// If the OS random source fails.
#[cfg(feature = "sign")]
pub fn generate_secret() -> Result<String, getrandom::Error> {
    let mut seed = [0_u8; 32];
    getrandom::fill(&mut seed)?;
    Ok(BASE64.encode(seed))
}

fn decode<const N: usize>(text: &str) -> Option<[u8; N]> {
    BASE64.decode(text.trim()).ok()?.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test-only key (all bytes 7), never used for releases.
    fn test_secret() -> String {
        BASE64.encode([7_u8; 32])
    }

    const NAME: &str = "OpenSeries-0.2.0-windows-x64-setup.exe";

    #[test]
    fn signature_round_trip() {
        let secret = test_secret();
        let public = public_key_of(&secret).unwrap();
        let signature = sign(&secret, NAME, b"package").unwrap();
        assert_eq!(verify_with(&public, NAME, b"package", &signature), Ok(()));
        // Trailing newline in the .sig file is fine.
        let with_newline = format!("{signature}\n");
        assert_eq!(
            verify_with(&public, NAME, b"package", &with_newline),
            Ok(())
        );
    }

    #[test]
    fn altered_package_or_name_is_rejected() {
        let secret = test_secret();
        let public = public_key_of(&secret).unwrap();
        let signature = sign(&secret, NAME, b"package").unwrap();
        assert_eq!(
            verify_with(&public, NAME, b"packagE", &signature),
            Err(SignatureError::Mismatch)
        );
        assert_eq!(
            verify_with(
                &public,
                "OpenSeries-9.9.9-windows-x64-setup.exe",
                b"package",
                &signature
            ),
            Err(SignatureError::Mismatch)
        );
    }

    #[test]
    fn foreign_key_and_garbage_are_rejected() {
        let signature = sign(&test_secret(), NAME, b"package").unwrap();
        let other = public_key_of(&BASE64.encode([9_u8; 32])).unwrap();
        assert_eq!(
            verify_with(&other, NAME, b"package", &signature),
            Err(SignatureError::Mismatch)
        );
        assert_eq!(
            verify_with(&other, NAME, b"package", "not base64!"),
            Err(SignatureError::Malformed)
        );
        assert_eq!(
            verify_with("short", NAME, b"package", &signature),
            Err(SignatureError::Key)
        );
    }

    #[test]
    fn built_in_key_is_valid() {
        assert!(decode::<32>(PUBLIC_KEY).is_some_and(|k| VerifyingKey::from_bytes(&k).is_ok()));
    }

    #[test]
    fn message_binds_name_and_digest() {
        let message = String::from_utf8(signed_message("a.zip", b"")).unwrap();
        assert_eq!(
            message,
            "openseries-update-v1\na.zip\n\
             e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
