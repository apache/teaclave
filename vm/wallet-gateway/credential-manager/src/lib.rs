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
use attestation::utils::{
    generate_end_cert, generate_inter_cert, generate_key_pair, load_pem_key,
    load_pem_key_from_bytes,
};
use crypto::sign_bytes_p384;
use db_manager::StorageClient;
use proto::BackupWalletInput;
use std::collections::HashSet;
use std::sync::Arc;
use storable::credential::{CaCredential, UserCredential};
use tls_client_processing::Credential;
use types::external::Email;
use types::share::{
    CkPrivateKey, CkPublicKey, CkSignature, DeviceID, TaUserInfo, TaWalletInfo, TeeConfig, WalletID,
};

const END_CERT_VAILD_SECONDS: i64 = 60 * 60 * 24 * 365; // 1 year
const INTER_CERT_VAILD_SECONDS: i64 = 60 * 60 * 24 * 365; // 1 year

pub struct CredentialManager {
    db_client: Arc<StorageClient>,
    ca_credential: CaCredential,
    cert_path: String,
}

impl CredentialManager {
    pub async fn init(db_client: Arc<StorageClient>, cert_path: &str) -> Result<Self> {
        let ca_creds = if let Some(ca_credential) = Self::load_from_db(db_client.clone()).await? {
            ca_credential
        } else {
            Self::load_from_file(cert_path)?
        };

        Ok(Self {
            db_client,
            ca_credential: ca_creds,
            cert_path: cert_path.to_string(),
        })
    }

    async fn load_from_db(db_client: Arc<StorageClient>) -> Result<Option<CaCredential>> {
        let ca_creds = db_client.get(&"CaCredential".into()).await?;
        Ok(ca_creds)
    }

    fn load_from_file(cert_path: &str) -> Result<CaCredential> {
        let ca_cert = std::fs::read(cert_path.to_owned() + "/ca.cert").map_err(|e| {
            anyhow!(
                "CaCredential::load: failed to read ca.cert from {}: {}",
                cert_path.to_owned() + "/ca.cert",
                e
            )
        })?;
        let ca_der = std::fs::read(cert_path.to_owned() + "/ca.der").map_err(|e| {
            anyhow!(
                "CaCredential::load: failed to read ca.der from {}: {}",
                cert_path.to_owned() + "/ca.der",
                e
            )
        })?;
        let ca_key = std::fs::read(cert_path.to_owned() + "/ca.key").map_err(|e| {
            anyhow!(
                "CaCredential::load: failed to read ca.key from {}: {}",
                cert_path.to_owned() + "/ca.key",
                e
            )
        })?;
        Ok(CaCredential::new(ca_cert, ca_der, ca_key))
    }

    fn ca_cert(&self) -> &Vec<u8> {
        self.ca_credential.ca_cert()
    }

    fn ca_der(&self) -> &Vec<u8> {
        self.ca_credential.ca_der()
    }

    pub async fn get_user_tls_credential(&self, email: &Email) -> Result<Credential> {
        let credential: UserCredential = match self.db_client.get(email).await? {
            Some(credential) => credential,
            None => {
                return Err(anyhow!(
                    "CredentialManager::get_user_tls_credential(): user {} not found",
                    email.0
                ))
            }
        };
        let client_der = credential.cert().clone();
        let client_key: CkPrivateKey = credential.into();
        Ok(Credential::new(
            self.ca_cert().clone(),
            client_der,
            client_key.into(),
            self.ca_der().clone(),
        ))
    }

    pub fn get_system_tls_credential(&self) -> Result<Credential> {
        let ca_cert = self.ca_cert().clone();
        let ca_der = self.ca_der().clone();
        let private_key = load_pem_key(&(self.cert_path.clone() + "/system.key")).map_err(|e| {
            anyhow!(
                "CredentialManager::get_system_tls_credential(): failed to load system.key: {}",
                e
            )
        })?;
        let cert = generate_end_cert(
            self.ca_credential.ca_key(),
            &private_key.0,
            "system",
            END_CERT_VAILD_SECONDS,
        )?;
        Ok(Credential::new(ca_cert, cert, private_key.0, ca_der))
    }

    pub async fn generate_user_credential(&self, email: &Email) -> Result<CkPublicKey> {
        // generate key pair
        let (public_key, private_key) = generate_key_pair()?;
        // generate cert
        let cert = generate_end_cert(
            self.ca_credential.ca_key(),
            &private_key,
            &email.0,
            END_CERT_VAILD_SECONDS,
        )?;

        let credential = UserCredential::new(
            email.clone(),
            CkPublicKey::new(public_key.as_slice()),
            CkPrivateKey::new(private_key.as_slice()),
            cert,
        );
        self.db_client.put(&credential).await?;
        Ok(CkPublicKey::new(public_key.as_slice()))
    }

    pub fn generate_tee_cert(&self, device_public_key: &CkPublicKey) -> Result<Vec<u8>> {
        // generate cert
        let tee_cert = generate_inter_cert(
            self.ca_credential.ca_key(),
            device_public_key.0.as_slice(),
            INTER_CERT_VAILD_SECONDS,
        )?;
        Ok(tee_cert)
    }

    pub fn sign_wallet_info(
        &self,
        user_registry: Vec<TaUserInfo>,
        wallets: Vec<TaWalletInfo>,
        config_version: u64,
    ) -> Result<TeeConfig> {
        let mut config = TeeConfig::new(user_registry, wallets, config_version);
        let priv_key = load_pem_key_from_bytes(self.ca_credential.ca_key())?;
        let sig = CkSignature::new(sign_bytes_p384(
            priv_key.0.as_slice(),
            &config.serialize()?,
        )?);
        config.set_signature(sig);
        Ok(config)
    }

    pub fn sign_backup_device_list(
        &self,
        backup_to_device_id: DeviceID,
        backup_to_device_pubkey: CkPublicKey,
        target_wallets: HashSet<WalletID>,
    ) -> Result<BackupWalletInput> {
        let mut input = BackupWalletInput::new_unsigned(
            backup_to_device_id,
            backup_to_device_pubkey,
            target_wallets,
        );
        let priv_key = load_pem_key_from_bytes(self.ca_credential.ca_key())?;
        let sig = CkSignature::new(sign_bytes_p384(priv_key.0.as_slice(), &input.serialize()?)?);
        input.set_signature(sig);
        Ok(input)
    }
}
