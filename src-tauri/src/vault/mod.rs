use crate::models::Result;
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rand::RngCore;
use zeroize::Zeroizing;
pub struct Vault {
    key: Option<Zeroizing<[u8; 32]>>,
    touched: std::time::Instant,
    pub interval: u64,
}
impl Default for Vault {
    fn default() -> Self {
        Self {
            key: None,
            touched: std::time::Instant::now(),
            interval: 300,
        }
    }
}
impl Vault {
    pub fn salt() -> String {
        let mut b = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut b);
        STANDARD.encode(b)
    }
    pub fn unlock(&mut self, password: &str, salt: &str, check: Option<&str>) -> Result<String> {
        self.lock();
        if password.len() < 10 {
            return Err("Use a passphrase of at least 10 characters".into());
        }
        let salt = STANDARD.decode(salt).map_err(|_| "Invalid vault salt")?;
        let mut key = Zeroizing::new([0u8; 32]);
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut *key)
            .map_err(|_| "Key derivation failed")?;
        self.key = Some(key);
        self.touched = std::time::Instant::now();
        if let Some(check) = check {
            if self.decrypt(check, "vault-check").as_deref() != Ok("NEXUS vault v1") {
                self.lock();
                return Err("Incorrect passphrase or damaged vault".into());
            }
        }
        self.encrypt("NEXUS vault v1", "vault-check")
    }
    pub fn lock(&mut self) {
        self.key = None;
    }
    pub fn unlocked(&mut self) -> bool {
        if self.touched.elapsed().as_secs() >= self.interval {
            self.lock()
        }
        self.key.is_some()
    }
    fn cipher(&mut self) -> Result<XChaCha20Poly1305> {
        if !self.unlocked() {
            return Err("Unlock the credential vault first".into());
        }
        self.touched = std::time::Instant::now();
        Ok(
            XChaCha20Poly1305::new_from_slice(self.key.as_ref().unwrap().as_ref())
                .expect("32 byte key"),
        )
    }
    pub fn encrypt(&mut self, value: &str, aad: &str) -> Result<String> {
        let cipher = self.cipher()?;
        let mut nonce = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut nonce);
        let encrypted = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: value.as_bytes(),
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| "Encryption failed")?;
        let mut out = nonce.to_vec();
        out.extend(encrypted);
        Ok(STANDARD.encode(out))
    }
    pub fn decrypt(&mut self, value: &str, aad: &str) -> Result<String> {
        let cipher = self.cipher()?;
        let bytes = STANDARD.decode(value).map_err(|_| "Invalid ciphertext")?;
        if bytes.len() < 40 {
            return Err("Invalid ciphertext".into());
        }
        let plain = cipher
            .decrypt(
                XNonce::from_slice(&bytes[..24]),
                Payload {
                    msg: &bytes[24..],
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| "Vault authentication failed")?;
        String::from_utf8(plain).map_err(|_| "Invalid secret encoding".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encryption_roundtrip_and_authentication() {
        let salt = Vault::salt();
        let mut v = Vault::default();
        let check = v.unlock("fictional-passphrase", &salt, None).unwrap();
        let c = v.encrypt("lab-only-secret", "credential-1").unwrap();
        assert!(!c.contains("lab-only"));
        assert_eq!(v.decrypt(&c, "credential-1").unwrap(), "lab-only-secret");
        assert!(v.decrypt(&c, "credential-2").is_err());
        v.lock();
        assert!(v.decrypt(&c, "credential-1").is_err());
        assert!(v.unlock("wrong-passphrase", &salt, Some(&check)).is_err());
        assert!(!v.unlocked());
        v.unlock("fictional-passphrase", &salt, Some(&check))
            .unwrap();
        assert_eq!(v.decrypt(&c, "credential-1").unwrap(), "lab-only-secret");
    }
}
