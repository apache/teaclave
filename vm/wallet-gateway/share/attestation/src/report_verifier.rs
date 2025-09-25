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

use crate::verify::{self, AttestationCertChain};
use anyhow::{ensure, Result};
use basic_utils::keccak_hash_to_bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[cfg(not(target_os = "optee"))]
use std::time::SystemTime;

#[cfg(target_os = "optee")]
use optee_utee::trace_println as println;
#[cfg(target_os = "optee")]
use rustls::optee_time::SystemTime;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct AttestationReport {
    pub measurement: Vec<u8>,
}
impl AttestationReport {
    pub fn from_cert(
        presented_certs: &[rustls::Certificate],
        report_ca_cert: &[u8],
    ) -> Result<Self> {
        // Extract information for attestation from TLS certification.
        ensure!(
            presented_certs.len() >= 3,
            "[+] AttestationReport: presented_certs.len() >= 3"
        );
        let end_cert = presented_certs[0].clone().0;
        let acc = AttestationCertChain {
            end: end_cert.clone(),
            intermediate: presented_certs[1].clone().0,
            root: presented_certs[2].clone().0,
        };
        verify::verify_cert_chain(acc, &rustls::Certificate(report_ca_cert.to_vec())).unwrap();
        let report = verify::extract_report_from_cert(&rustls::Certificate(end_cert)).unwrap();
        Ok(report)
    }
}
impl Default for AttestationReport {
    fn default() -> Self {
        Self {
            measurement: vec![0u8; 32],
        }
    }
}

#[derive(Debug, Deserialize, Clone, Eq, PartialEq)]
pub struct EnclaveAttr {
    pub measurement: Vec<u8>,
}

/// User defined verification function to further verify the attestation report.
pub type AttestationReportVerificationFn = fn(&AttestationReport) -> bool;

/// Type used to verify attestation reports (this can be set as a certificate
/// verifier in `rustls::ClientConfig`).
#[derive(Clone)]
pub struct AttestationReportVerifier {
    /// Valid enclave attributes (only enclaves with attributes in this vector
    /// will be accepted).
    pub accepted_enclave_attrs: Vec<EnclaveAttr>,
    /// Root certificate of the attestation service provider (e.g., IAS).
    pub root_ca: Vec<u8>,
    /// User defined function to verify the attestation report.
    pub verifier: AttestationReportVerificationFn,
}

impl AttestationReportVerifier {
    pub fn new(
        accepted_enclave_attrs: Vec<EnclaveAttr>,
        root_ca: &[u8],
        verifier: AttestationReportVerificationFn,
    ) -> Self {
        Self {
            accepted_enclave_attrs,
            root_ca: root_ca.to_vec(),
            verifier,
        }
    }

    /// Verify whether the `MR_SIGNER` and `MR_ENCLAVE` in the attestation report is
    /// accepted by us, which are defined in `accepted_enclave_attrs`.
    fn verify_measures(&self, _attestation_report: &AttestationReport) -> bool {
        println!("[+] AttestationReportVerifier::verify measures");
        true
    }

    /// Verify TLS certificate.
    fn verify_server_cert(&self, presented_certs: &[rustls::Certificate]) -> bool {
        println!("[+] AttestationReportVerifier::verify server cert");

        let report = match AttestationReport::from_cert(presented_certs, &self.root_ca) {
            Ok(report) => report,
            Err(e) => {
                println!(
                    "[+] AttestationReportVerifier::cert verification error {:?}",
                    e
                );
                return false;
            }
        };
        println!(
            "[+] AttestationReportVerifier::extract report successfully, report: {:?}",
            &report
        );

        self.verify_measures(&report) && (self.verifier)(&report)
    }
}

impl rustls::client::ServerCertVerifier for AttestationReportVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        // This call automatically verifies certificate signature
        println!("[+] ServerCertVerifier::verify server cert");
        let mut presented_certs = intermediates.to_vec();
        presented_certs.insert(0, end_entity.clone());
        if self.verify_server_cert(presented_certs.as_slice()) {
            Ok(rustls::client::ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "server cert verification failed".to_string(),
            ))
        }
    }
}

/// Type used to verify client certificates from server side
pub type ClientVerifierFn = fn(&[u8]) -> bool;
#[derive(Clone)]
pub struct ClientVerifier {
    pub accepted_pubkey_hash: HashSet<Vec<u8>>,
    pub verifier_fn: ClientVerifierFn,
}
impl ClientVerifier {
    pub fn new(accepted_pubkey_hash: HashSet<Vec<u8>>, verifier_fn: ClientVerifierFn) -> Self {
        Self {
            accepted_pubkey_hash,
            verifier_fn,
        }
    }
}

impl rustls::server::ClientCertVerifier for ClientVerifier {
    fn offer_client_auth(&self) -> bool {
        true
    }

    #[allow(deprecated)]
    fn client_auth_root_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        now: SystemTime,
    ) -> Result<rustls::server::ClientCertVerified, rustls::Error> {
        #[cfg(not(target_os = "optee"))]
        let _now =
            webpki::Time::try_from(now).map_err(|_| rustls::Error::FailedToGetCurrentTime)?;
        #[cfg(target_os = "optee")]
        let _now = webpki::Time::from_seconds_since_unix_epoch(now.secs());

        println!("[+] ClientVerifier: invoke verify_client_cert");
        // extract pubkey from client cert
        // check if pub key exists in accepted_pub_keys
        let pub_key = match verify::extract_pub_key_from_cert(end_entity) {
            Ok(pub_key) => pub_key,
            Err(e) => {
                println!(
                    "[+] ClientVerifier: extract_pub_key_from_cert error {:?}",
                    e
                );
                return Err(rustls::Error::General(
                    "client cert verification failed".to_string(),
                ));
            }
        };
        println!("[+] ClientVerifier: pub_key: {:?}", pub_key);

        // since attestation is a independent module, it doesn't import the "types"
        // so we implement the hash function here
        let pubkey_hash = keccak_hash_to_bytes(&pub_key)[..8].to_vec();
        if self.accepted_pubkey_hash.contains(&pubkey_hash) {
            return Ok(rustls::server::ClientCertVerified::assertion());
        }

        println!("[+] ClientVerifier: pub_key is not authorized");
        Err(rustls::Error::HandshakeNotComplete)
    }
}
