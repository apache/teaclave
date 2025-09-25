// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use anyhow::{anyhow, Result};
use basic_utils::generate_random_bytes;
use ring::aead;
use serde::{Deserialize, Serialize};

// AES_GCM 128
// encrypting/decrypting data-key
pub const AES_GCM_128_KEY_LENGTH: usize = 16;
pub const AES_GCM_128_IV_LENGTH: usize = 12;
pub const CMAC_LENGTH: usize = 16;
type CMac = [u8; CMAC_LENGTH];

/// The above code defines a struct for AES-GCM 128-bit encryption and decryption with methods for
/// encrypting and decrypting data and generating a CMAC.
///
/// Properties:
///
/// * `key`: A 128-bit AES key used for encryption and decryption in the AES-GCM mode of operation.
/// * `iv`: `iv` stands for initialization vector. It is a fixed-size array of bytes used as an input to
/// the AES-GCM encryption algorithm to ensure that the same plaintext message encrypted with the same
/// key produces different ciphertexts. The `iv` is typically randomly generated for each encryption
/// operation and must be kept
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AesGcm128Key {
    pub key: [u8; AES_GCM_128_KEY_LENGTH],
    pub iv: [u8; AES_GCM_128_IV_LENGTH],
}

/// This is an implementation of methods for the `AesGcm128Key` struct.
impl AesGcm128Key {
    pub const _SCHEMA: &'static str = "aes-gcm-128";

    /// This function creates a new instance of AesGcm128Key with the given key and initialization vector.
    ///
    /// Arguments:
    ///
    /// * `in_key`: in_key is a reference to a byte slice that represents the encryption key used in
    /// AES-GCM-128 encryption. The length of the slice should be 16 bytes (AES_GCM_128_KEY_LENGTH).
    /// * `in_iv`: `in_iv` is a slice of bytes representing the initialization vector (IV) used in AES-GCM
    /// encryption. The IV is a random value that is used to ensure that each encryption operation produces
    /// a unique ciphertext, even if the same plaintext is encrypted multiple times. The length of the IV is
    /// typically
    ///
    /// Returns:
    ///
    /// The `new` function is returning a `Result` containing an `AesGcm128Key` struct.
    pub fn new(in_key: &[u8], in_iv: &[u8]) -> Result<Self> {
        let mut key = [0u8; AES_GCM_128_KEY_LENGTH];
        let mut iv = [0u8; AES_GCM_128_IV_LENGTH];
        key.copy_from_slice(in_key);
        iv.copy_from_slice(in_iv);

        Ok(AesGcm128Key { key, iv })
    }

    pub fn generate() -> Result<Self> {
        let key = generate_random_bytes(AES_GCM_128_KEY_LENGTH)?;
        let iv = generate_random_bytes(AES_GCM_128_IV_LENGTH)?;

        Ok(AesGcm128Key {
            key: key.try_into().map_err(|_| anyhow!("try_into error"))?,
            iv: iv.try_into().map_err(|_| anyhow!("try_into error"))?,
        })
    }

    /// This function decrypts a message using AES-128-GCM and returns a CMAC authentication tag.
    ///
    /// Arguments:
    ///
    /// * `in_out`: `in_out` is a mutable reference to a `Vec<u8>` that contains the ciphertext to be
    /// decrypted. The decrypted plaintext will be written back to the same `Vec<u8>` in place of the
    /// ciphertext.
    ///
    /// Returns:
    ///
    /// a `Result` containing a `CMac` value.
    pub fn decrypt(&self, in_out: &mut Vec<u8>) -> Result<CMac> {
        let plaintext_len = aead_decrypt(&aead::AES_128_GCM, in_out, &self.key, &self.iv)?.len();
        let mut cmac: CMac = [0u8; CMAC_LENGTH];
        cmac.copy_from_slice(&in_out[plaintext_len..]);
        in_out.truncate(plaintext_len);
        Ok(cmac)
    }

    /// This function encrypts a given input using AES-128-GCM and returns a CMAC value.
    ///
    /// Arguments:
    ///
    /// * `in_out`: `in_out` is a mutable reference to a vector of bytes (`Vec<u8>`). This vector contains
    /// the plaintext to be encrypted and will also be used to store the resulting ciphertext. The function
    /// will modify this vector in place to encrypt the plaintext and append the resulting CMAC
    /// (Cipher-based Message
    ///
    /// Returns:
    ///
    /// a `Result` containing a `CMac` value.
    pub fn encrypt(&self, in_out: &mut Vec<u8>) -> Result<CMac> {
        aead_encrypt(&aead::AES_128_GCM, in_out, &self.key, &self.iv)?;
        let mut cmac: CMac = [0u8; CMAC_LENGTH];
        let n = in_out.len();
        let cybertext_len = n - CMAC_LENGTH;
        cmac.copy_from_slice(&in_out[cybertext_len..]);
        Ok(cmac)
    }
}

/// This function performs AEAD encryption on a given input using a specified algorithm, key, and
/// initialization vector.
///
/// Arguments:
///
/// * `alg`: a reference to an AEAD (Authenticated Encryption with Associated Data) algorithm, which is
/// used for encrypting and authenticating data.
/// * `in_out`: `in_out` is a mutable reference to a vector of bytes that contains the plaintext to be
/// encrypted. After encryption, the ciphertext will be appended to this vector.
/// * `key`: The `key` parameter is a byte slice that contains the secret key used for encryption. It is
/// used to create an `UnboundKey` object, which is then used to create a `LessSafeKey` object for
/// encryption.
/// * `iv`: `iv` stands for initialization vector. It is a fixed-size input to the encryption algorithm
/// that is typically used to ensure that the same plaintext input does not produce the same ciphertext
/// output. The `iv` should be unique for each encryption operation.
///
/// Returns:
///
/// a `Result` type, which can either be `Ok(())` if the encryption was successful or an error if
/// something went wrong during the encryption process.
pub fn aead_encrypt(
    alg: &'static aead::Algorithm,
    in_out: &mut Vec<u8>,
    key: &[u8],
    iv: &[u8],
) -> Result<()> {
    let key =
        aead::UnboundKey::new(alg, key).map_err(|_| anyhow!("aead::UnboundKey::new() Error"))?;
    let nonce = aead::Nonce::try_assume_unique_for_key(iv)
        .map_err(|_| anyhow!("aead::Nonce::try_assume_unique_for_key() Error"))?;
    let aad = aead::Aad::from([0u8; 8]);

    let enc_key = aead::LessSafeKey::new(key);
    enc_key
        .seal_in_place_append_tag(nonce, aad, in_out)
        .map_err(|_| anyhow!("aead::LessSafeKey::seal_in_place_append_tag() Error"))?;
    Ok(())
}

/// This function performs authenticated encryption with associated data (AEAD) decryption using a given
/// algorithm, key, initialization vector, and input/output buffer.
///
/// Arguments:
///
/// * `alg`: The AEAD (Authenticated Encryption with Associated Data) algorithm to be used for
/// decryption.
/// * `in_out`: `in_out` is a mutable reference to a byte slice that contains the ciphertext to be
/// decrypted. The function will modify this slice in place to store the decrypted plaintext.
/// * `key`: The `key` parameter is a byte slice representing the secret key used for decryption. It is
/// used to create an `UnboundKey` object, which is then used to create a `LessSafeKey` object for
/// decryption.
/// * `iv`: `iv` stands for initialization vector. It is a fixed-size input to the encryption algorithm
/// that is typically used to ensure that the same plaintext input does not produce the same ciphertext
/// output. It is also used to derive the initial state of the encryption algorithm. In the context of
/// this function, `iv
///
/// Returns:
///
/// a `Result` that contains a mutable reference to the input/output buffer (`&'a mut [u8]`) if the
/// decryption is successful. If there is an error during decryption, the function will return an error.
pub fn aead_decrypt<'a>(
    alg: &'static aead::Algorithm,
    in_out: &'a mut [u8],
    key: &[u8],
    iv: &[u8],
) -> Result<&'a mut [u8]> {
    let key =
        aead::UnboundKey::new(alg, key).map_err(|_| anyhow!("aead::UnboundKey::new() Error"))?;
    let nonce = aead::Nonce::try_assume_unique_for_key(iv)
        .map_err(|_| anyhow!("aead::Nonce::try_assume_unique_for_key() Error"))?;
    let aad = aead::Aad::from([0u8; 8]);

    let dec_key = aead::LessSafeKey::new(key);
    let slice = dec_key
        .open_in_place(nonce, aad, in_out)
        .map_err(|_| anyhow!("aead::LessSafeKey::open_in_place() Error"))?;
    Ok(slice)
}
