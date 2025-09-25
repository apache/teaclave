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

use crate::user_registry::UserRegistry;
use credential_manager::CredentialManager;
use storable::console::{ConsoleWalletInfo, WalletAccountInfo, MAX_WALLET_NUM_PER_DEVICE};

use db_manager::StorageClient;
use storable::device_info::{BackupDeviceInfoBasic, DeviceIndex, DeviceInfo, DeviceInfoBasic};
use types::external::Email;
use types::share::{
    AccountId, CkHasher, CkPublicKey, DeviceID, Role, TaUserInfo, TaWalletInfo, TeeConfig, WalletID,
};

use anyhow::{anyhow, bail, ensure, Result};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

pub struct DeviceRegistry {
    db_client: Arc<StorageClient>,
}
impl DeviceRegistry {
    pub async fn init(db_client: Arc<StorageClient>) -> Result<Self> {
        Ok(Self { db_client })
    }

    // device_index.db operations
    async fn update_device_index(&mut self, device_index: &DeviceIndex) -> Result<()> {
        // update db
        self.db_client.put(device_index).await?;
        Ok(())
    }
    pub async fn get_device_index_for_user(&self, email: &Email) -> Result<Option<DeviceIndex>> {
        self.db_client.get(email).await
    }
    pub async fn device_exist_for_user(&self, email: &Email, device_id: &DeviceID) -> bool {
        match self.get_device_index_for_user(email).await {
            Ok(Some(device_index)) => device_index.device_list().contains(device_id),
            _ => false,
        }
    }
    pub async fn device_owner(&self, device_id: &DeviceID) -> Result<Email> {
        for (email, device_index) in self.db_client.list_entries::<Email, DeviceIndex>().await? {
            if device_index.device_list().contains(device_id) {
                return Ok(email);
            }
        }
        bail!(
            "DeviceRegistry::device_owner(): device {:?} not exist in db",
            &device_id
        );
    }

    // command RegisterDevice
    pub async fn register_device(
        &mut self,
        credential_manager: &CredentialManager,
        email: &Email,
        signing_pubkey: CkPublicKey,
        backup_pubkey: CkPublicKey,
    ) -> Result<DeviceInfoBasic> {
        log::debug!("register_device(): pubkey: {:?}", &signing_pubkey);
        let device_id = DeviceID::from(signing_pubkey.clone());
        log::info!("register device {:?} for user {:?}", &device_id, &email);
        match self.device_owner(&device_id).await {
            Ok(owner) => {
                bail!(
                    "DeviceRegistry::register_device(): device {:?} already exists for user {:?}",
                    &device_id,
                    &owner
                );
            }
            Err(_) => {
                log::debug!(
                    "DeviceRegistry::register_device(): device {:?} does not exist in db",
                    &device_id
                );
            }
        };
        let device_tee_cert = credential_manager.generate_tee_cert(&signing_pubkey)?;
        let device_info = DeviceInfo::new(
            email.clone(),
            device_id.clone(),
            signing_pubkey,
            backup_pubkey,
            device_tee_cert,
            HashSet::new(),
            HashSet::new(),
        );
        // update device index
        let new_device_list = match self.get_device_index_for_user(email).await? {
            Some(device_index) => {
                let mut device_list = device_index.device_list().clone();
                device_list.insert(device_id);
                device_list
            }
            None => {
                let mut device_list = HashSet::new();
                device_list.insert(device_id);
                device_list
            }
        };
        self.update_device_index(&DeviceIndex::new(email.clone(), new_device_list))
            .await?;

        // update device info db
        self.db_client.put(&device_info).await?;
        Ok(device_info.into())
    }

    // command ExportDeviceCert
    pub async fn export_device_cert(&self, device_id: &DeviceID) -> Result<Vec<u8>> {
        let device_info: DeviceInfo = match self.db_client.get(device_id).await? {
            Some(device_info) => device_info,
            None => {
                bail!(
                    "DeviceRegistry::export_device_cert(): device {:?} not exist in db",
                    &device_id
                );
            }
        };
        Ok(device_info.cert().clone())
    }

    // command RefreshDeviceCert
    pub async fn refresh_device_cert(
        &mut self,
        credential_manager: &CredentialManager,
        device_id: &DeviceID,
    ) -> Result<Vec<u8>> {
        let mut device_info: DeviceInfo = match self.db_client.get(device_id).await? {
            Some(device_info) => device_info,
            None => {
                bail!(
                    "DeviceRegistry::refresh_device_cert(): device {:?} not exist in db",
                    &device_id
                );
            }
        };
        let device_tee_cert = credential_manager.generate_tee_cert(device_info.signing_pubkey())?;
        device_info.set_cert(device_tee_cert);
        self.db_client.put(&device_info).await?;

        Ok(device_info.cert().clone())
    }

    // command AuthorizeBackupDevice
    pub async fn export_backup_device_list(
        &self,
        credential_manager: &CredentialManager,
        from_device_id: &DeviceID,
        backup_device_info_basic: &BackupDeviceInfoBasic,
    ) -> Result<(Vec<u8>, u64)> {
        let from_device_info: DeviceInfo = match self.db_client.get(from_device_id).await? {
            Some(device_info) => device_info,
            None => {
                bail!(
                    "DeviceRegistry::export_backup_device_list(): device {:?} not exist in db",
                    &from_device_id
                );
            }
        };
        let mut to_device_info: DeviceInfo = match self
            .db_client
            .get(&backup_device_info_basic.device_id)
            .await?
        {
            Some(device_info) => device_info,
            None => {
                bail!(
                    "DeviceRegistry::export_backup_device_list(): device {:?} not exist in db",
                    &backup_device_info_basic.device_id
                );
            }
        };

        // if wallet_id_list is empty, backup all wallets of this device
        let wallet_id_list = if backup_device_info_basic.wallet_id_list.is_empty() {
            log::info!(
                "DeviceRegistry::export_backup_device_list(): backup all wallets of device {:?}",
                from_device_id
            );
            from_device_info.created_wallets().iter().cloned().collect()
        } else {
            log::info!(
                "DeviceRegistry::export_backup_device_list(): backup wallets {:?} of device {:?}",
                &backup_device_info_basic.wallet_id_list,
                from_device_id
            );
            backup_device_info_basic.wallet_id_list.clone()
        };

        for wallet_id in &wallet_id_list {
            let mut wallet_info: ConsoleWalletInfo = match self.db_client.get(wallet_id).await? {
                Some(wallet_info) => wallet_info,
                None => {
                    bail!(
                        "DeviceRegistry::export_backup_device_list(): wallet {:?} not exist in db",
                        &wallet_id
                    );
                }
            };
            ensure!(
                    from_device_info.created_wallets().contains(wallet_id),
                    "DeviceRegistry::export_backup_device_list(): check device_info: wallet {:?} not owned by device {:?}",
                    wallet_id,
                    from_device_info.id(),
                );
            ensure!(
                    &wallet_info.created_by_device == from_device_id,
                    "DeviceRegistry::export_backup_device_list(): check wallet_info: wallet {:?} not owned by device {:?}",
                    wallet_id,
                    &from_device_id
                );
            ensure!(
                    to_device_info.existing_wallets_num() < MAX_WALLET_NUM_PER_DEVICE,
                    "DeviceRegistry::export_backup_device_list(): wallet num exceed max wallet num per device",
                );

            wallet_info.add_backup_device(to_device_info.id().clone());
            self.db_client.put(&wallet_info).await?;

            log::info!("DeviceRegistry::export_backup_device_list(): wallet_info: saved in db");
            to_device_info.add_backup_wallet(wallet_id)?;
        }
        self.db_client.put(&to_device_info).await?;
        log::info!("DeviceRegistry::export_backup_device_list(): to_device_info: saved in db");

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow!("Failed to get timestamp: {}", e))?
            .as_secs();

        // Empty list means backup all wallets: the TEE will handle this case
        let wallet_id_list_to_signed = if backup_device_info_basic.wallet_id_list.is_empty() {
            HashSet::new()
        } else {
            to_device_info.backed_up_wallets().clone()
        };
        let input = credential_manager.sign_backup_device_list(
            to_device_info.id().clone(),
            to_device_info.backup_pubkey().clone(),
            wallet_id_list_to_signed,
        )?;

        let serialized_backup_wallet_input = bincode::serialize(&input)?;

        Ok((serialized_backup_wallet_input, timestamp))
    }

    // command RegisterWallet
    pub async fn register_wallet(
        &mut self,
        user_registry: Arc<RwLock<UserRegistry>>,
        device_id: &DeviceID,
        wallet_info_basic: &ConsoleWalletInfo,
    ) -> Result<WalletID> {
        let mut device_info: DeviceInfo = match self.db_client.get(device_id).await? {
            Some(device_info) => device_info,
            None => {
                bail!(
                    "DeviceRegistry::register_wallet(): device {:?} not exist in db",
                    &device_id
                );
            }
        };
        // check if the wallet id has been registered
        match self
            .db_client
            .get::<_, ConsoleWalletInfo>(wallet_info_basic.id())
            .await?
        {
            Some(wallet_info) => {
                bail!(
                    "DeviceRegistry::register_wallet(): wallet {:?} already exists for device {:?}",
                    wallet_info.wallet_id,
                    wallet_info.created_by_device,
                );
            }
            None => {
                log::debug!(
                    "DeviceRegistry::register_wallet(): wallet {:?} does not exist in db",
                    wallet_info_basic.id()
                );
            }
        };
        // check user's role
        for stage in wallet_info_basic.approval_chain_basic.iter() {
            for approvers in stage.approvers.iter() {
                let user_info = user_registry.read().await.get_info(approvers).await?;
                if !user_info.get_roles().contains(&Role::Approver) {
                    bail!(
                        "DeviceRegistry::register_wallet(): user {:?} is not approver",
                        approvers
                    );
                };
            }
        }
        for tx_operator in wallet_info_basic.authorized_operators.clone() {
            let user_info = user_registry.read().await.get_info(&tx_operator).await?;
            if !user_info.get_roles().contains(&Role::TxOperator) {
                bail!(
                    "DeviceRegistry::register_wallet(): user {:?} is not tx_operator",
                    tx_operator
                );
            };
        }

        // check max wallet num per device
        ensure!(
            device_info.existing_wallets_num() < MAX_WALLET_NUM_PER_DEVICE,
            "DeviceRegistry::add_wallet_for_device(): max wallet num per device reached"
        );

        device_info.add_owned_wallet(wallet_info_basic)?;
        self.db_client.put(wallet_info_basic).await?;
        self.db_client.put(&device_info).await?;
        log::info!(
            "DeviceRegistry::add_wallet_for_device(): wallet_info and device_info saved in db"
        );
        Ok(wallet_info_basic.id().clone())
    }

    pub async fn remove_wallet(&mut self, wallet_id: &WalletID) -> Result<()> {
        let device_id = match self
            .db_client
            .get::<_, ConsoleWalletInfo>(wallet_id)
            .await?
        {
            Some(wallet_info) => wallet_info.created_by_device.clone(),
            None => {
                bail!(
                    "DeviceRegistry::remove_wallet(): wallet {:?} not exist in db",
                    &wallet_id
                );
            }
        };
        let mut device_info: DeviceInfo = match self.db_client.get(&device_id).await? {
            Some(device_info) => device_info,
            None => {
                bail!(
                    "DeviceRegistry::remove_wallet(): device {:?} not exist in db",
                    &device_id
                );
            }
        };
        device_info.remove_owned_wallet(wallet_id)?;
        self.db_client
            .delete_entry::<WalletID, ConsoleWalletInfo>(wallet_id)
            .await?;
        self.db_client.put(&device_info).await?;
        Ok(())
    }

    // command ExportWalletInfo
    pub async fn export_wallet_info(
        &self,
        credential_manager: &CredentialManager,
        user_registry: Arc<RwLock<UserRegistry>>,
        device_id: &DeviceID,
    ) -> Result<(TeeConfig, u64)> {
        let device_info: DeviceInfo = match self.db_client.get(device_id).await? {
            Some(device_info) => device_info,
            None => {
                bail!(
                    "DeviceRegistry::export_wallet_info(): device {:?} not exist in db",
                    &device_id
                );
            }
        };
        let wallet_id_list = device_info.wallet_id_list();
        log::info!(
            "DeviceRegistry::export_wallet_info(): wallet_id_list: {:?}",
            wallet_id_list
        );

        // gather all wallets
        // convert to wallets_info_ta
        // gather all participants
        // gather all user pubkeys to registry

        let mut wallets = Vec::new();
        for wallet_id in &wallet_id_list {
            let wallet_info_basic: ConsoleWalletInfo = match self.db_client.get(wallet_id).await? {
                Some(wallet_info) => wallet_info,
                None => {
                    bail!(
                        "DeviceRegistry::export_wallet_info(): wallet {:?} not exist in db",
                        &wallet_id
                    );
                }
            };
            wallets.push(wallet_info_basic);
        }

        let all_related_users: HashSet<&Email> = wallets
            .iter()
            .flat_map(|wallet| wallet.distinct_participants())
            .collect();

        let mut registry: Vec<TaUserInfo> = Vec::new();
        for each_user in all_related_users {
            let user_info = user_registry.read().await.get_info(each_user).await?;
            let ta_user_info = TaUserInfo::new(
                user_info.get_pub_key().hash()?,
                each_user.clone().into(),
                user_info.get_roles().clone(),
            );
            registry.push(ta_user_info);
        }

        let ta_wallets: Vec<TaWalletInfo> =
            wallets.into_iter().map(|wallet| wallet.into()).collect();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow!("Failed to get timestamp: {}", e))?
            .as_secs();

        let input = credential_manager.sign_wallet_info(registry, ta_wallets, timestamp)?;
        Ok((input, timestamp))
    }

    // command GetUserInfo
    pub async fn get_device_info_for_user(&self, email: &Email) -> Result<Vec<DeviceInfoBasic>> {
        let mut device_info_list: Vec<DeviceInfoBasic> = Vec::new();
        let device_index = self.get_device_index_for_user(email).await?.ok_or(anyhow!(
            "DeviceRegistry::get_device_info_for_user(): user {:?} not exist in db",
            &email
        ))?;
        let device_id_list = device_index.device_list();
        for device_id in device_id_list {
            let device_info: DeviceInfo = match self.db_client.get(device_id).await? {
                Some(device_info) => device_info,
                None => {
                    bail!(
                        "DeviceRegistry::get_device_info_for_user(): device {:?} not exist in db",
                        &device_id
                    );
                }
            };
            let device_info_for_user: DeviceInfoBasic = device_info.into();
            device_info_list.push(device_info_for_user);
        }
        Ok(device_info_list)
    }

    // command GetWalletInfo
    pub async fn get_wallet_info(&self, wallet_id: &WalletID) -> Result<Option<ConsoleWalletInfo>> {
        self.db_client.get::<_, ConsoleWalletInfo>(wallet_id).await
    }

    // command UpdateWalletInfo
    pub async fn save_wallet_info(&self, wallet_info_basic: &ConsoleWalletInfo) -> Result<()> {
        self.db_client.put(wallet_info_basic).await
    }

    // command GetDeviceInfo
    pub async fn get_device_info(&self, device_id: &DeviceID) -> Result<Option<DeviceInfoBasic>> {
        match self.db_client.get::<_, DeviceInfo>(device_id).await? {
            Some(device_info) => Ok(Some(device_info.into())),
            None => Ok(None),
        }
    }

    pub async fn get_wallet_id_associated_with_device(
        &self,
        device_id: &DeviceID,
    ) -> Result<Vec<WalletID>> {
        let mut wallet_id_list = Vec::new();
        let device_info: DeviceInfo = match self.db_client.get(device_id).await? {
            Some(device_info) => device_info,
            None => {
                bail!(
                    "DeviceRegistry::get_wallet_id_associated_with_device(): device {:?} not exist in db",
                    &device_id
                );
            }
        };
        wallet_id_list.extend(device_info.created_wallets().clone());
        wallet_id_list.extend(device_info.backed_up_wallets().clone());
        Ok(wallet_id_list)
    }

    pub async fn get_wallet_account(&self, wallet_id: &WalletID) -> Result<Option<AccountId>> {
        match self
            .db_client
            .get::<WalletID, WalletAccountInfo>(wallet_id)
            .await?
        {
            Some(wallet_account_info) => Ok(Some(wallet_account_info.account_address)),
            None => Ok(None),
        }
    }

    pub async fn save_wallet_account(
        &self,
        wallet_id: &WalletID,
        account: &AccountId,
    ) -> Result<()> {
        let wallet_account_info = WalletAccountInfo::new(wallet_id.clone(), account.clone());
        self.db_client.put(&wallet_account_info).await
    }
}
