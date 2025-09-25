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
use attestation::report_verifier::{AttestationReport, AttestationReportVerifier, EnclaveAttr};
use attestation::utils::load_pem_cert_from_bytes;
use proto::{TaCommand, TlsCommandRequest};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::string::String;
use std::sync::Arc;

fn server_verifier(report: &AttestationReport) -> bool {
    log::info!("[+] server_verifier: attestation report: {:?}", report);
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Credential {
    inter_cert_chain: Vec<u8>, // [inter_cert, ca_cert]
    client_der: Vec<u8>,       // client's cert in der format
    client_private_key: Vec<u8>,
    server_ca_der: Vec<u8>, // for verifying server's certificate
}

impl Credential {
    pub fn new(
        inter_cert_chain: Vec<u8>,
        client_der: Vec<u8>,
        client_private_key: Vec<u8>,
        server_ca_der: Vec<u8>,
    ) -> Self {
        Credential {
            inter_cert_chain,
            client_der,
            client_private_key,
            server_ca_der,
        }
    }
    pub(crate) fn client_cert_chain(&self) -> Result<Vec<rustls::Certificate>> {
        let mut inter_cert_chain = load_pem_cert_from_bytes(&self.inter_cert_chain)?;
        let client_cert = rustls::Certificate(self.client_der.clone());
        inter_cert_chain.insert(0, client_cert);
        Ok(inter_cert_chain)
    }
    pub(crate) fn server_ca_cert(&self) -> Result<Vec<u8>> {
        Ok(self.server_ca_der.clone())
    }
}

impl From<Credential> for rustls::PrivateKey {
    fn from(cred: Credential) -> Self {
        rustls::PrivateKey(cred.client_private_key)
    }
}

pub struct TlsClient {
    tls_stream: Option<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>,
    config: Arc<rustls::ClientConfig>,
    server_name: String,
    server_addr: String,
}

impl TlsClient {
    pub fn new(credential: Credential, url: &url::Url) -> Result<Self> {
        let server_ip = url
            .host_str()
            .ok_or_else(|| anyhow!("Invalid server name: {:?}", url.host_str()))?;
        let server_port = url
            .port()
            .ok_or_else(|| anyhow!("Invalid port: {:?}", url.port()))?;
        let server_name = server_ip.to_string();
        let server_addr = format!("{}:{}", server_ip, server_port);

        // set server verifier
        let accepted_enclave_attrs = vec![EnclaveAttr {
            measurement: vec![0u8],
        }];
        let verifier = Arc::new(AttestationReportVerifier::new(
            accepted_enclave_attrs,
            &credential.server_ca_cert()?,
            server_verifier,
        ));

        let mut root_store = rustls::RootCertStore::empty();
        if let Some(root_cert) = credential.client_cert_chain()?.last() {
            root_store.add_parsable_certificates(&[root_cert.0.clone()]);
        } else {
            return Err(anyhow!(
                "tls_connect: invalid cert. client_cert_chain is empty"
            ));
        }

        let mut config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_single_cert(credential.client_cert_chain()?, credential.into())?;
        config.dangerous().set_certificate_verifier(verifier);

        let config = Arc::new(config);

        let mut client = TlsClient {
            tls_stream: None,
            config,
            server_name,
            server_addr,
        };
        client.reconnect()?;
        Ok(client)
    }

    fn reconnect(&mut self) -> Result<()> {
        let conn = rustls::ClientConnection::new(
            self.config.clone(),
            self.server_name.as_str().try_into()?,
        )?;
        let sock = TcpStream::connect(&self.server_addr)?;
        let tls = rustls::StreamOwned::new(conn, sock);
        self.tls_stream = Some(tls);
        log::info!("[+] client: tls handshake finished (reconnect)");
        Ok(())
    }

    fn tls_connect(&mut self, request: Vec<u8>) -> Result<Vec<u8>> {
        match send_tls_request(&mut self.tls_stream, &request) {
            Ok(resp) => Ok(resp),
            Err(e) => {
                log::warn!(
                    "[+] tls-client-processing: connection error, will try to reconnect: {}",
                    e
                );
                self.reconnect()?;
                send_tls_request(&mut self.tls_stream, &request)
            }
        }
    }

    pub fn invoke_command(&mut self, command: TaCommand, input: &[u8]) -> Result<Vec<u8>> {
        log::info!("[+] invoke_command: command: {:?}", command);
        let command_request = TlsCommandRequest {
            command,
            request: input.to_vec(),
        };
        let serialized_command_request = bincode::serialize(&command_request)?;
        log::debug!(
            "[+] invoke_command: serialized_command_request len: {}",
            serialized_command_request.len()
        );
        // invoke tls
        let output = self.tls_connect(serialized_command_request)?;
        log::info!("[+] invoke_command: tls_connect finished");
        Ok(output)
    }

    pub fn invoke<I, O>(&mut self, input: I, command: TaCommand) -> Result<O>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let input_buffer = bincode::serialize(&input)?;
        let output_buffer = match self.invoke_command(command, &input_buffer) {
            Ok(output) => output,
            Err(e) => {
                anyhow::bail!(
                    "processing_threshold_command: {:?} error. TEE doesn't return output. Error: {}",
                    command,
                    e
                );
            }
        };
        log::debug!("[+] invoke: output_buffer len: {}", &output_buffer.len());
        let output: O = match bincode::deserialize(&output_buffer) {
            Ok(output) => output,
            Err(e) => {
                log::error!(
                "processing_threshold_command: {:?} error. TEE doesn't return output. Error: {}",
                command,
                e
            );
                anyhow::bail!(
                    "processing_threshold_command: {:?} error. TEE error message: {:?}",
                    command,
                    String::from_utf8_lossy(&output_buffer)
                );
            }
        };
        Ok(output)
    }
}

fn send_tls_request(
    tls_stream: &mut Option<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>,
    request: &[u8],
) -> Result<Vec<u8>> {
    let tls_stream = tls_stream
        .as_mut()
        .ok_or_else(|| anyhow!("TLS stream not connected"))?;
    tls_stream.write_all(request)?;
    tls_stream.flush()?;

    let mut response = Vec::new();
    let mut chunk = [0u8; 1024 * 10];
    let n = tls_stream.read(&mut chunk)?;
    response.extend_from_slice(&chunk[..n]);
    Ok(response)
}
