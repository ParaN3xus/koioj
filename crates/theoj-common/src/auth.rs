use std::str::FromStr;

use crate::error::{Context, Result};
use ssh_key::{Algorithm, HashAlg, PrivateKey, PublicKey, SshSig};

pub fn load_private_key(path: &str) -> Result<PrivateKey> {
    let expanded_path = shellexpand::tilde(path);
    let key_data = std::fs::read_to_string(expanded_path.as_ref())
        .context("Failed to read private key file")?;

    PrivateKey::from_openssh(&key_data).context("Failed to parse private key")
}

pub fn load_public_key(path: &str) -> Result<PublicKey> {
    let expanded_path = shellexpand::tilde(path);
    let key_data = std::fs::read_to_string(expanded_path.as_ref())
        .context("Failed to read public key file")?;

    PublicKey::from_openssh(&key_data).context("Failed to parse public key")
}

pub fn sign_message(private_key: &PrivateKey, message: String) -> Result<String> {
    tracing::debug!("Private key algorithm: {:?}", private_key.algorithm());
    tracing::debug!("Private key is_encrypted: {}", private_key.is_encrypted());

    let hash_alg = match private_key.algorithm() {
        Algorithm::Rsa { hash: Some(h) } => h,
        Algorithm::Rsa { hash: None } => HashAlg::Sha512,
        _ => HashAlg::default(),
    };

    let signature = private_key
        .sign("file", hash_alg, message.as_bytes())
        .context("Failed to sign message")?;

    Ok(signature.to_pem(ssh_key::LineEnding::LF)?)
}

pub fn verify_signature(public_key: &PublicKey, message: &[u8], signature: String) -> Result<()> {
    let sig = SshSig::from_str(&signature).context("Invalid signature format")?;
    public_key
        .verify("file", message, &sig)
        .context("Signature verification failed")?;
    Ok(())
}
pub fn create_challenge(judge_id: &str, timestamp: i64) -> String {
    format!("{}:{}", judge_id, timestamp)
}
