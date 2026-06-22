use base64::{engine::general_purpose, Engine as _};
use md5::{Digest, Md5};
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};

const FINALSHELL_HEAD_LEN: usize = 8;
const DES_BLOCK_LEN: usize = 8;
const DES_KEY_LEN: usize = 8;
const JAVA_RANDOM_MULTIPLIER: u64 = 0x5DEECE66D;
const JAVA_RANDOM_ADDEND: u64 = 0xB;
const JAVA_RANDOM_MASK: u64 = (1_u64 << 48) - 1;
const FINALSHELL_KEY_SEED: i64 = 3_680_984_568_597_093_857;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum FinalShellPasswordError {
    #[error("please input encrypted FinalShell password")]
    EmptyInput,
    #[error("FinalShell password must be Base64 text")]
    InvalidBase64,
    #[error("FinalShell password payload is too short")]
    PayloadTooShort,
    #[error("FinalShell password key seed is invalid")]
    InvalidKeySeed,
    #[error("FinalShell password cipher text length is invalid")]
    InvalidCipherLength,
    #[error("FinalShell password padding is invalid")]
    InvalidPadding,
    #[error("FinalShell password decrypted text is not valid UTF-8")]
    InvalidUtf8,
}

pub fn decode_finalshell_password(input: &str) -> Result<String, FinalShellPasswordError> {
    let encrypted = input.trim();
    if encrypted.is_empty() {
        return Err(FinalShellPasswordError::EmptyInput);
    }

    let payload = general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|_| FinalShellPasswordError::InvalidBase64)?;
    if payload.len() <= FINALSHELL_HEAD_LEN {
        return Err(FinalShellPasswordError::PayloadTooShort);
    }

    let mut head = [0_u8; FINALSHELL_HEAD_LEN];
    head.copy_from_slice(&payload[..FINALSHELL_HEAD_LEN]);
    let cipher_text = &payload[FINALSHELL_HEAD_LEN..];
    let key = Zeroizing::new(random_des_key(&head)?);
    let plain_text = Zeroizing::new(des_ecb_decrypt(cipher_text, &key)?);
    let unpadded = remove_pkcs5_padding(plain_text.as_slice())?;

    std::str::from_utf8(unpadded)
        .map(str::to_string)
        .map_err(|_| FinalShellPasswordError::InvalidUtf8)
}

fn random_des_key(
    head: &[u8; FINALSHELL_HEAD_LEN],
) -> Result<[u8; DES_KEY_LEN], FinalShellPasswordError> {
    let divisor = JavaRandom::new(signed_byte(head[5])).next_int(127);
    if divisor == 0 {
        return Err(FinalShellPasswordError::InvalidKeySeed);
    }

    let key_seed = FINALSHELL_KEY_SEED / i64::from(divisor);
    let mut random = JavaRandom::new(key_seed);
    let skip_count = i32::from(head[0] as i8).max(0);
    for _ in 0..skip_count {
        random.next_long();
    }

    let random_seed = random.next_long();
    let mut secondary_random = JavaRandom::new(random_seed);
    let key_longs = [
        signed_byte(head[4]),
        secondary_random.next_long(),
        signed_byte(head[7]),
        signed_byte(head[3]),
        secondary_random.next_long(),
        signed_byte(head[1]),
        random.next_long(),
        signed_byte(head[2]),
    ];
    let mut key_data = Zeroizing::new(Vec::with_capacity(key_longs.len() * 8));
    for value in key_longs {
        key_data.extend_from_slice(&value.to_be_bytes());
    }

    let mut digest = Md5::digest(key_data.as_slice());
    let mut key = [0_u8; DES_KEY_LEN];
    key.copy_from_slice(&digest[..DES_KEY_LEN]);
    digest.zeroize();

    Ok(key)
}

fn signed_byte(value: u8) -> i64 {
    i64::from(value as i8)
}

#[derive(Debug)]
struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ JAVA_RANDOM_MULTIPLIER) & JAVA_RANDOM_MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self
            .seed
            .wrapping_mul(JAVA_RANDOM_MULTIPLIER)
            .wrapping_add(JAVA_RANDOM_ADDEND)
            & JAVA_RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32 as i32
    }

    fn next_int(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            return 0;
        }

        if (bound & -bound) == bound {
            return ((i64::from(bound) * i64::from(self.next(31))) >> 31) as i32;
        }

        loop {
            let bits = self.next(31);
            let value = bits % bound;
            if bits - value + (bound - 1) >= 0 {
                return value;
            }
        }
    }

    fn next_long(&mut self) -> i64 {
        (i64::from(self.next(32)) << 32).wrapping_add(i64::from(self.next(32)))
    }
}

fn des_ecb_decrypt(
    cipher_text: &[u8],
    key: &[u8; DES_KEY_LEN],
) -> Result<Vec<u8>, FinalShellPasswordError> {
    if cipher_text.is_empty() || cipher_text.len() % DES_BLOCK_LEN != 0 {
        return Err(FinalShellPasswordError::InvalidCipherLength);
    }

    let subkeys = build_des_subkeys(key);
    let mut plain_text = Vec::with_capacity(cipher_text.len());
    for chunk in cipher_text.chunks_exact(DES_BLOCK_LEN) {
        let mut block = [0_u8; DES_BLOCK_LEN];
        block.copy_from_slice(chunk);
        plain_text.extend_from_slice(&des_decrypt_block(block, &subkeys));
    }

    Ok(plain_text)
}

fn remove_pkcs5_padding(plain_text: &[u8]) -> Result<&[u8], FinalShellPasswordError> {
    let Some(&padding_len) = plain_text.last() else {
        return Err(FinalShellPasswordError::InvalidPadding);
    };
    let padding_len = usize::from(padding_len);
    if padding_len == 0 || padding_len > DES_BLOCK_LEN || padding_len > plain_text.len() {
        return Err(FinalShellPasswordError::InvalidPadding);
    }

    if plain_text[plain_text.len() - padding_len..]
        .iter()
        .any(|&value| usize::from(value) != padding_len)
    {
        return Err(FinalShellPasswordError::InvalidPadding);
    }

    Ok(&plain_text[..plain_text.len() - padding_len])
}

fn build_des_subkeys(key: &[u8; DES_KEY_LEN]) -> [u64; 16] {
    let permuted_key = permute(u64::from_be_bytes(*key), 64, &DES_PC1);
    let mut left = (permuted_key >> 28) & 0x0FFF_FFFF;
    let mut right = permuted_key & 0x0FFF_FFFF;
    let mut subkeys = [0_u64; 16];

    for (index, shift) in DES_SHIFTS.iter().enumerate() {
        left = rotate_left_28(left, *shift);
        right = rotate_left_28(right, *shift);
        subkeys[index] = permute((left << 28) | right, 56, &DES_PC2);
    }

    subkeys
}

fn des_decrypt_block(block: [u8; DES_BLOCK_LEN], subkeys: &[u64; 16]) -> [u8; DES_BLOCK_LEN] {
    let permuted_block = permute(u64::from_be_bytes(block), 64, &DES_IP);
    let mut left = (permuted_block >> 32) as u32;
    let mut right = permuted_block as u32;

    for index in (0..16).rev() {
        let next_left = right;
        right = left ^ des_feistel(right, subkeys[index]);
        left = next_left;
    }

    permute((u64::from(right) << 32) | u64::from(left), 64, &DES_FP).to_be_bytes()
}

fn des_feistel(right: u32, subkey: u64) -> u32 {
    let expanded = permute(u64::from(right), 32, &DES_EXPANSION);
    let mixed = expanded ^ subkey;
    let mut sbox_result = 0_u32;

    for (index, sbox) in DES_SBOXES.iter().enumerate() {
        let shift = 42 - (index * 6);
        let chunk = ((mixed >> shift) & 0x3F) as u8;
        let row = ((chunk & 0x20) >> 4) | (chunk & 0x01);
        let column = (chunk >> 1) & 0x0F;
        let value = sbox[usize::from(row * 16 + column)];
        sbox_result = (sbox_result << 4) | u32::from(value);
    }

    permute(u64::from(sbox_result), 32, &DES_P) as u32
}

fn rotate_left_28(value: u64, shift: u8) -> u64 {
    ((value << shift) | (value >> (28 - shift))) & 0x0FFF_FFFF
}

fn permute(input: u64, input_bits: u8, table: &[u8]) -> u64 {
    let mut output = 0_u64;
    for &position in table {
        let bit = (input >> u32::from(input_bits - position)) & 1;
        output = (output << 1) | bit;
    }
    output
}

const DES_IP: [u8; 64] = [
    58, 50, 42, 34, 26, 18, 10, 2, 60, 52, 44, 36, 28, 20, 12, 4, 62, 54, 46, 38, 30, 22, 14, 6,
    64, 56, 48, 40, 32, 24, 16, 8, 57, 49, 41, 33, 25, 17, 9, 1, 59, 51, 43, 35, 27, 19, 11, 3, 61,
    53, 45, 37, 29, 21, 13, 5, 63, 55, 47, 39, 31, 23, 15, 7,
];

const DES_FP: [u8; 64] = [
    40, 8, 48, 16, 56, 24, 64, 32, 39, 7, 47, 15, 55, 23, 63, 31, 38, 6, 46, 14, 54, 22, 62, 30,
    37, 5, 45, 13, 53, 21, 61, 29, 36, 4, 44, 12, 52, 20, 60, 28, 35, 3, 43, 11, 51, 19, 59, 27,
    34, 2, 42, 10, 50, 18, 58, 26, 33, 1, 41, 9, 49, 17, 57, 25,
];

const DES_EXPANSION: [u8; 48] = [
    32, 1, 2, 3, 4, 5, 4, 5, 6, 7, 8, 9, 8, 9, 10, 11, 12, 13, 12, 13, 14, 15, 16, 17, 16, 17, 18,
    19, 20, 21, 20, 21, 22, 23, 24, 25, 24, 25, 26, 27, 28, 29, 28, 29, 30, 31, 32, 1,
];

const DES_P: [u8; 32] = [
    16, 7, 20, 21, 29, 12, 28, 17, 1, 15, 23, 26, 5, 18, 31, 10, 2, 8, 24, 14, 32, 27, 3, 9, 19,
    13, 30, 6, 22, 11, 4, 25,
];

const DES_PC1: [u8; 56] = [
    57, 49, 41, 33, 25, 17, 9, 1, 58, 50, 42, 34, 26, 18, 10, 2, 59, 51, 43, 35, 27, 19, 11, 3, 60,
    52, 44, 36, 63, 55, 47, 39, 31, 23, 15, 7, 62, 54, 46, 38, 30, 22, 14, 6, 61, 53, 45, 37, 29,
    21, 13, 5, 28, 20, 12, 4,
];

const DES_PC2: [u8; 48] = [
    14, 17, 11, 24, 1, 5, 3, 28, 15, 6, 21, 10, 23, 19, 12, 4, 26, 8, 16, 7, 27, 20, 13, 2, 41, 52,
    31, 37, 47, 55, 30, 40, 51, 45, 33, 48, 44, 49, 39, 56, 34, 53, 46, 42, 50, 36, 29, 32,
];

const DES_SHIFTS: [u8; 16] = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];

const DES_SBOXES: [[u8; 64]; 8] = [
    [
        14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7, 0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12,
        11, 9, 5, 3, 8, 4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0, 15, 12, 8, 2, 4, 9,
        1, 7, 5, 11, 3, 14, 10, 0, 6, 13,
    ],
    [
        15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10, 3, 13, 4, 7, 15, 2, 8, 14, 12, 0, 1,
        10, 6, 9, 11, 5, 0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15, 13, 8, 10, 1, 3, 15,
        4, 2, 11, 6, 7, 12, 0, 5, 14, 9,
    ],
    [
        10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8, 13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5,
        14, 12, 11, 15, 1, 13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7, 1, 10, 13, 0, 6,
        9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12,
    ],
    [
        7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15, 13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2,
        12, 1, 10, 14, 9, 10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4, 3, 15, 0, 6, 10, 1,
        13, 8, 9, 4, 5, 11, 12, 7, 2, 14,
    ],
    [
        2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9, 14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15,
        10, 3, 9, 8, 6, 4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14, 11, 8, 12, 7, 1, 14,
        2, 13, 6, 15, 0, 9, 10, 4, 5, 3,
    ],
    [
        12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11, 10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13,
        14, 0, 11, 3, 8, 9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6, 4, 3, 2, 12, 9, 5,
        15, 10, 11, 14, 1, 7, 6, 0, 8, 13,
    ],
    [
        4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1, 13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5,
        12, 2, 15, 8, 6, 1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2, 6, 11, 13, 8, 1, 4,
        10, 7, 9, 5, 0, 15, 14, 2, 3, 12,
    ],
    [
        13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7, 1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6,
        11, 0, 14, 9, 2, 7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8, 2, 1, 14, 7, 4, 10,
        8, 13, 15, 12, 9, 0, 3, 5, 6, 11,
    ],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_sample_finalshell_password() {
        let decoded =
            decode_finalshell_password("eGANOUNaAUlE5cPFQXrtPfyej430A+k2ruM2JPtvU/I=").unwrap();

        assert_eq!(decoded, "Ben1993YYDSchina855!@#");
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(
            decode_finalshell_password("   ").unwrap_err(),
            FinalShellPasswordError::EmptyInput
        );
    }

    #[test]
    fn rejects_non_base64_input() {
        assert_eq!(
            decode_finalshell_password("not a base64 value").unwrap_err(),
            FinalShellPasswordError::InvalidBase64
        );
    }

    #[test]
    fn rejects_short_payload() {
        assert_eq!(
            decode_finalshell_password("MTIzNDU2Nzg=").unwrap_err(),
            FinalShellPasswordError::PayloadTooShort
        );
    }

    #[test]
    fn decrypts_des_known_answer_vector() {
        let key = [0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1];
        let cipher_text = [0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05];
        let plain_text = des_decrypt_block(cipher_text, &build_des_subkeys(&key));

        assert_eq!(plain_text, [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]);
    }
}
