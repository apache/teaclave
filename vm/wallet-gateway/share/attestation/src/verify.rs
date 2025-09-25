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

use crate::{report_verifier::AttestationReport, utils::get_webpki_time};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct CertInfo {
    pub issuer: String,
    pub subject: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct AttestationCertChain {
    pub end: Vec<u8>,
    pub intermediate: Vec<u8>,
    pub root: Vec<u8>,
}

type SignatureAlgorithms = &'static [&'static webpki::SignatureAlgorithm];
static SUPPORTED_SIG_ALGS: SignatureAlgorithms = &[
    &webpki::ECDSA_P256_SHA256,
    &webpki::ECDSA_P256_SHA384,
    &webpki::ECDSA_P384_SHA256,
    &webpki::ECDSA_P384_SHA384,
];

/// Construct a AttestationReport from a X509 certificate and verify
/// attestation report with the report_ca_cert which is from the attestation
/// service provider.
pub fn extract_report_from_cert(cert: &rustls::Certificate) -> Result<AttestationReport> {
    use crate::cert::*;
    let cert = &cert.0;
    // Extract information for attestation from TLS certification.
    let x509 = yasna::parse_der(cert, X509::load).unwrap();
    let tbs_cert: <TbsCert as Asn1Ty>::ValueTy = x509.0;
    let pub_key: <PubKey as Asn1Ty>::ValueTy = ((((((tbs_cert.1).1).1).1).1).1).0;
    let _pub_k = (pub_key.1).0;
    let cert_ext: <RaCertExt as Asn1Ty>::ValueTy = (((((((tbs_cert.1).1).1).1).1).1).1).0 .0;
    let cert_ext_payload: Vec<u8> = (cert_ext.1).0;

    // Convert to endorsed report
    let report = serde_json::from_slice(&cert_ext_payload)?;
    Ok(report)
}

/// extract issuer and subject from certificate
pub fn extract_info_from_cert(cert: &rustls::Certificate) -> Result<CertInfo> {
    use crate::cert::*;
    let cert = &cert.0;
    // Extract information from certificate
    let x509 = yasna::parse_der(cert, X509::load).unwrap();
    let tbs_cert: <TbsCert as Asn1Ty>::ValueTy = x509.0;
    let issuer: <Issuer as Asn1Ty>::ValueTy = (((tbs_cert.1).1).1).0;
    let issuer_payload = (((issuer.0).0).1).0;
    let subject: <Subject as Asn1Ty>::ValueTy = (((((tbs_cert.1).1).1).1).1).0;
    let subject_payload = (((subject.0).0).1).0;

    Ok(CertInfo {
        issuer: issuer_payload,
        subject: subject_payload,
    })
}

/// extract public key from certificate
pub fn extract_pub_key_from_cert(cert: &rustls::Certificate) -> Result<Vec<u8>> {
    use crate::cert::*;
    let cert = &cert.0;
    // Extract information from certificate
    let x509 = yasna::parse_der(cert, X509::load).unwrap();
    let tbs_cert: <TbsCert as Asn1Ty>::ValueTy = x509.0;
    let pub_key: <PubKey as Asn1Ty>::ValueTy = ((((((tbs_cert.1).1).1).1).1).1).0;
    let pub_k = (pub_key.1).0;

    Ok(pub_k.to_bytes())
}

pub fn verify_cert_chain(acc: AttestationCertChain, ca_cert: &rustls::Certificate) -> Result<()> {
    let end_cert = webpki::EndEntityCert::try_from(acc.end.as_slice())?;
    let anchor = webpki::TrustAnchor::try_from_cert_der(ca_cert.0.as_ref())
        .map_err(|_| anyhow!("invalid root cert"))?;
    let trust_anchors = vec![anchor];

    let chain = vec![&acc.intermediate[..], &acc.root[..]];
    let time = get_webpki_time()?;
    end_cert.verify_is_valid_tls_server_cert(
        SUPPORTED_SIG_ALGS,
        &webpki::TlsServerTrustAnchors(&trust_anchors),
        &chain,
        time,
    )?;
    Ok(())
}
