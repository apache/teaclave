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
use ring::rand;
use ring::signature::{self, EcdsaSigningAlgorithm, KeyPair, ECDSA_P384_SHA384_ASN1};
pub use ring::signature::{
    ECDSA_P256_SHA256_ASN1, ECDSA_P256_SHA256_ASN1_SIGNING, ECDSA_P384_SHA384_ASN1_SIGNING,
};

/// EcdsaKeyPair stores a pair of ECDSA (private, public) key based on the
/// NIST P-256 curve (a.k.a secp256r1).
pub struct EcdsaKeyPair {
    key_pair: signature::EcdsaKeyPair,
    pkcs8_bytes: Vec<u8>,
}

impl EcdsaKeyPair {
    /// Generate a ECDSA key pair.
    pub fn new() -> Result<Self> {
        let rng = rand::SystemRandom::new();
        let pkcs8_bytes = signature::EcdsaKeyPair::generate_pkcs8(
            &signature::ECDSA_P256_SHA256_ASN1_SIGNING,
            &rng,
        )
        .unwrap();
        let key_pair = signature::EcdsaKeyPair::from_pkcs8(
            &signature::ECDSA_P256_SHA256_ASN1_SIGNING,
            pkcs8_bytes.as_ref(),
        )
        .unwrap();
        Ok(Self {
            key_pair,
            pkcs8_bytes: pkcs8_bytes.as_ref().to_vec(),
        })
    }

    pub fn from_bytes(pkcs8_bytes: &[u8], algo: &'static EcdsaSigningAlgorithm) -> Result<Self> {
        let key_pair = signature::EcdsaKeyPair::from_pkcs8(algo, pkcs8_bytes).unwrap();
        Ok(Self {
            key_pair,
            pkcs8_bytes: pkcs8_bytes.to_vec(),
        })
    }

    pub fn pub_key(&self) -> &[u8] {
        self.key_pair.public_key().as_ref()
    }

    pub fn prv_key(&self) -> &[u8] {
        self.pkcs8_bytes.as_ref()
    }

    pub fn sign(&self, msg: &[u8]) -> Result<Vec<u8>> {
        let rng = rand::SystemRandom::new();
        match self.key_pair.sign(&rng, msg) {
            Ok(sig) => Ok(sig.as_ref().to_vec()),
            Err(_) => Err(anyhow!("failed to sign")),
        }
    }
}

pub fn sign_bytes_p256(priv_key: &[u8], bytes: &[u8]) -> Result<Vec<u8>> {
    let key_pair = EcdsaKeyPair::from_bytes(priv_key, &ECDSA_P256_SHA256_ASN1_SIGNING)?;
    let sig = key_pair.sign(bytes)?;
    Ok(sig.to_vec())
}

pub fn verify_signature_p256(pub_key: &[u8], bytes: &[u8], sig: &[u8]) -> Result<()> {
    let peer_pub_key = signature::UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, pub_key);
    match peer_pub_key.verify(bytes, sig) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("signature verification failed: {:?}", e)),
    }
}

pub fn sign_bytes_p384(priv_key: &[u8], bytes: &[u8]) -> Result<Vec<u8>> {
    let key_pair = EcdsaKeyPair::from_bytes(priv_key, &ECDSA_P384_SHA384_ASN1_SIGNING)?;
    let sig = key_pair.sign(bytes)?;
    Ok(sig.to_vec())
}

pub fn verify_signature_p384(pub_key: &[u8], bytes: &[u8], sig: &[u8]) -> Result<()> {
    let peer_pub_key = signature::UnparsedPublicKey::new(&ECDSA_P384_SHA384_ASN1, pub_key);
    match peer_pub_key.verify(bytes, sig) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!("signature verification failed: {:?}", e)),
    }
}
