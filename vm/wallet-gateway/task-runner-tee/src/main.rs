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

mod task;

use ck_config::RuntimeConfig;
use credential_manager::CredentialManager;
use db_manager::{DBCompatibleClient, LocalServiceClient, StorageClient};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use types::share::{CkNetwork, NetworkType};

use net::{BtcRpcEndpoint, EthRpcEndpoint};
use tls_client_processing::TlsClient;
use types::external::Erc20TokenConfig;
use utils::logger::setup_logger;

use task_exec::Executable;

#[tokio::main]
async fn main() {
    let root_path_str = std::env::var("CK_ROOT_PATH")
        .map_err(|e| {
            log::error!("CK_ROOT_PATH error: {:?}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })
        .expect("CK_ROOT_PATH not set");
    let root_path = std::path::Path::new(&root_path_str);
    let config_file_path = std::path::Path::new("config/runtime_config.toml");
    let task_config = TaskConfig::new(root_path, config_file_path);

    setup_logger(&task_config.log_path, "task-runner-tee").expect("Failed to initialize logger");

    let db_client = Arc::new(StorageClient::new(Box::new(
        LocalServiceClient::init(task_config.db_client_address.as_str(), None)
            .await
            .expect("failed to init db client"),
    )));

    let credential_manager =
        CredentialManager::init(db_client.clone(), task_config.cert_path.to_str().unwrap())
            .await
            .expect("failed to init credential manager");
    log::debug!("credential manager initialized");

    let tls_client = Arc::new(RwLock::new(
        TlsClient::new(
            credential_manager
                .get_system_tls_credential()
                .expect("failed to get system tls credential"),
            &task_config.tee_address,
        )
        .expect("failed to init tls client"),
    ));

    let infura_url = CkNetwork::Eth(task_config.network_type)
        .rpc_api_url()
        .join(&task_config.infura_token)
        .unwrap();
    let erc20_abi = Erc20TokenConfig::new(task_config.network_type)
        .unwrap()
        .erc20_abi()
        .to_owned();
    let ethereum_rpc = Arc::new(EthRpcEndpoint::new(
        infura_url,
        Some(erc20_abi.clone()),
        task_config.network_type,
    ));
    let bitcoin_rpc = Arc::new(BtcRpcEndpoint::new(CkNetwork::Btc(
        task_config.network_type,
    )));
    let bsc_api_url = CkNetwork::Bsc(task_config.network_type).rpc_api_url();
    let bsc_rpc = Arc::new(EthRpcEndpoint::new(
        bsc_api_url,
        Some(erc20_abi),
        task_config.network_type,
    ));

    let sync_tee_status_task = task::SyncTeeStatusTask::new(db_client.clone(), tls_client.clone());
    let sync_wallet_info_task =
        task::SyncWalletInfoTask::new(db_client.clone(), tls_client.clone());

    let tx_delegation_task = task::TxDelegationTask::new(
        db_client.clone(),
        tls_client.clone(),
        ethereum_rpc,
        bsc_rpc,
        bitcoin_rpc,
    );

    task_exec::join_periodic_tasks!(
        task_config.task_interval,
        sync_tee_status_task,
        sync_wallet_info_task,
        tx_delegation_task
    );
}

struct TaskConfig {
    task_interval: u64,
    db_client_address: url::Url,
    cert_path: PathBuf,
    log_path: PathBuf,
    tee_address: url::Url,
    network_type: NetworkType,
    infura_token: String,
}

impl TaskConfig {
    pub fn new(root_path: &Path, config_file_path: &Path) -> Self {
        let runtime_config = RuntimeConfig::from_toml(root_path.join(config_file_path))
            .expect("failed to read runtime config");

        let infura_token = std::env::var("INFURA_TOKEN").expect("INFURA_TOKEN not set");
        let network_type = runtime_config.blockchain_network.network_type;

        let task_interval = runtime_config.task.task_exec_intervals;
        let db_client_address = runtime_config
            .internal_endpoints
            .db_service
            .advertised_address;
        let cert_path = root_path.join(runtime_config.storage.certs_path);
        let log_path = root_path.join(runtime_config.storage.log_path);
        let tee_address = runtime_config
            .internal_endpoints
            .tee_wallet
            .advertised_address;

        Self {
            task_interval,
            db_client_address,
            cert_path,
            log_path,
            tee_address,
            infura_token,
            network_type,
        }
    }
}
