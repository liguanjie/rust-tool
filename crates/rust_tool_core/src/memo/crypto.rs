use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

/// Derive a 32-byte AES key from a password and salt using PBKDF2-HMAC-SHA256.
pub fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    // 100k iterations is a good balance between speed and security for a local desktop app
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut key);
    key
}

/// Encrypt data using AES-256-GCM, returning "IV_base64:Ciphertext_base64"
pub fn encrypt(data: &[u8], key: &[u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new(key.into());
    let mut iv = [0u8; 12];
    use aes_gcm::aead::rand_core::RngCore;
    OsRng.fill_bytes(&mut iv);
    let nonce = Nonce::from_slice(&iv);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("Encryption error: {:?}", e))?;

    let iv_b64 = BASE64_STANDARD.encode(iv);
    let cipher_b64 = BASE64_STANDARD.encode(ciphertext);
    Ok(format!("{}:{}", iv_b64, cipher_b64))
}

/// Decrypt an encrypted string formatted as "IV_base64:Ciphertext_base64"
pub fn decrypt(encrypted_str: &str, key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let parts: Vec<&str> = encrypted_str.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid encrypted string format".to_string());
    }

    let iv = BASE64_STANDARD
        .decode(parts[0])
        .map_err(|e| format!("Failed to decode IV: {:?}", e))?;
    let ciphertext = BASE64_STANDARD
        .decode(parts[1])
        .map_err(|e| format!("Failed to decode ciphertext: {:?}", e))?;

    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(&iv);

    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("Decryption error: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let password = "my_master_password";
        let salt = b"some_random_salt_bytes";
        let key = derive_key(password, salt);

        let plaintext = b"database_password_123456";
        let encrypted = encrypt(plaintext, &key).expect("Encryption failed");
        assert!(encrypted.contains(':'));

        let decrypted = decrypt(&encrypted, &key).expect("Decryption failed");
        assert_eq!(plaintext.to_vec(), decrypted);
    }
}
