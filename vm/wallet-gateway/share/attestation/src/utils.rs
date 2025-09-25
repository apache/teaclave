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

use crate::report_verifier::AttestationReport;
use crate::sign::{sign_csr_with_extension, sign_intermediate};
use basic_utils::get_system_time_seconds;
use crypto::{EcdsaKeyPair, ECDSA_P256_SHA256_ASN1_SIGNING, ECDSA_P384_SHA384_ASN1_SIGNING};

use anyhow::{Context, Result};
use std::convert::TryInto;
use std::fs;
use std::io::BufReader;

pub fn load_pem_cert(filename: &str) -> Result<Vec<rustls::Certificate>> {
    let certfile = fs::File::open(filename).with_context(|| "cannot open certificate file")?;
    let mut reader = BufReader::new(certfile);
    let certs = rustls_pemfile::certs(&mut reader)
        .map_err(|_| anyhow::anyhow!("pemfile::load_pem_cert failed"));
    Ok(certs
        .unwrap()
        .iter()
        .map(|v| rustls::Certificate(v.clone()))
        .collect())
}

pub fn load_pem_single_cert(filename: &str) -> Result<rustls::Certificate> {
    let certs = load_pem_cert(filename)?;
    assert!(certs.len() == 1);
    Ok(certs[0].clone())
}

pub fn load_pem_cert_from_bytes(bytes: &[u8]) -> Result<Vec<rustls::Certificate>> {
    let cursor = std::io::Cursor::new(bytes);
    let mut reader = BufReader::new(cursor);
    let certs = rustls_pemfile::certs(&mut reader)
        .map_err(|_| anyhow::anyhow!("pemfile::load_pem_cert failed"));
    Ok(certs
        .unwrap()
        .iter()
        .map(|v| rustls::Certificate(v.clone()))
        .collect())
}

pub fn load_pem_key(filename: &str) -> Result<rustls::PrivateKey> {
    let keyfile = fs::File::open(filename).with_context(|| "cannot open private key file")?;
    let mut reader = BufReader::new(keyfile);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .map_err(|_| anyhow::anyhow!("pemfile::pkcs8_private_keys failed"))?;
    assert!(keys.len() == 1);
    Ok(rustls::PrivateKey(keys[0].clone()))
}

pub fn load_pem_key_from_bytes(bytes: &[u8]) -> Result<rustls::PrivateKey> {
    let cursor = std::io::Cursor::new(bytes);
    let mut reader = BufReader::new(cursor);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .map_err(|_| anyhow::anyhow!("pemfile::pkcs8_private_keys failed"))?;
    assert!(keys.len() == 1);
    Ok(rustls::PrivateKey(keys[0].clone()))
}

pub fn generate_end_cert(
    ca_key: &[u8],
    end_key: &[u8],
    user_name: &str,
    cert_valid_seconds: i64,
) -> Result<Vec<u8>> {
    let ca_key = load_pem_key_from_bytes(ca_key)?;
    let ca_kp = EcdsaKeyPair::from_bytes(&ca_key.0, &ECDSA_P384_SHA384_ASN1_SIGNING)?;
    let end_kp = EcdsaKeyPair::from_bytes(end_key, &ECDSA_P256_SHA256_ASN1_SIGNING)?;
    let end_pub = end_kp.pub_key();
    assert_eq!(
        end_pub[0], 0x4,
        "Missing uncompressed public key indicator."
    );

    let report = AttestationReport::default();
    let payload = serde_json::to_vec(&report).unwrap();
    let end_der = sign_csr_with_extension(
        &ca_kp,
        end_pub,
        "CK ECDSA CA",
        &("CK client: ".to_string() + user_name),
        ["client", "localhost"],
        &payload,
        cert_valid_seconds,
    )?;
    Ok(end_der)
}

pub fn generate_inter_cert(
    ca_key: &[u8],
    inter_pub_key: &[u8],
    cert_valid_seconds: i64,
) -> Result<Vec<u8>> {
    let ca_key = load_pem_key_from_bytes(ca_key)?;
    let ca_kp = EcdsaKeyPair::from_bytes(&ca_key.0, &ECDSA_P384_SHA384_ASN1_SIGNING)?;
    assert!(
        inter_pub_key[0] == 0x4,
        "Missing uncompressed public key indicator."
    );

    let inter_der = sign_intermediate(
        &ca_kp,
        inter_pub_key,
        "CK ECDSA CA",
        "CK ECDSA level 2 intermediate",
        cert_valid_seconds,
    )?;
    Ok(inter_der)
}
pub fn generate_key_pair() -> Result<(Vec<u8>, Vec<u8>)> {
    let key_pair = EcdsaKeyPair::new()?;
    let priv_key = key_pair.prv_key();
    let pub_key = key_pair.pub_key();
    Ok((pub_key.to_vec(), priv_key.to_vec()))
}

pub fn get_webpki_time() -> Result<webpki::Time> {
    let time = webpki::Time::from_seconds_since_unix_epoch(
        get_system_time_seconds()?
            .try_into()
            .map_err(|_| anyhow::anyhow!("time conversion failed"))?,
    );
    Ok(time)
}
