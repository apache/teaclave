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

use ck_config::RuntimeConfig;
use db_manager::{DBCompatibleClient, LocalServiceClient, StorageClient};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use net::{
    AssetPriceEndpoint, BtcRpcEndpoint, EthReceivedPaymentEndpoint, EthRpcEndpoint,
    MailServiceEndpoint,
};
use task_exec::Executable;
use types::external::Erc20TokenConfig;
use types::share::{CkNetwork, NetworkType};
use utils::logger::setup_logger;

mod task;
use crate::task::{
    EmailNotifyTask, SyncAssetPriceTask, SyncBalanceInfoTask, SyncGasPriceTask,
    SyncPaymentInfoTask, SyncUtxosTask,
};

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

    setup_logger(&task_config.log_path, "task-runner-normal").expect("Failed to initialize logger");

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
        Some(erc20_abi.clone()),
        task_config.network_type,
    ));

    let db_client = Arc::new(StorageClient::new(Box::new(
        LocalServiceClient::init(task_config.db_client_address.as_str(), None)
            .await
            .expect("failed to init db client"),
    )));

    // price info task runner
    let ft_sync_price_task = SyncGasPriceTask::new(
        db_client.clone(),
        ethereum_rpc.clone(),
        bsc_rpc.clone(),
        bitcoin_rpc.clone(),
        task_config.network_type,
    );

    // balance info task runner
    let ft_sync_balance_task =
        SyncBalanceInfoTask::new(db_client.clone(), ethereum_rpc.clone(), bsc_rpc.clone());

    // currency info task runner
    let ft_sync_currency_task = SyncAssetPriceTask::new(
        db_client.clone(),
        AssetPriceEndpoint::new(
            task_config.currency_query_base_url,
            task_config.currency_query_token,
        ),
    );

    let eth_explorer = EthReceivedPaymentEndpoint::new(
        CkNetwork::Eth(task_config.network_type),
        task_config.etherscan_token,
    );
    let bsc_explorer = EthReceivedPaymentEndpoint::new(
        CkNetwork::Bsc(task_config.network_type),
        task_config.bscscan_token,
    );
    // received payment task runner
    let ft_sync_received_payment_task = SyncPaymentInfoTask::new(
        db_client.clone(),
        eth_explorer,
        bsc_explorer,
        task_config.network_type,
    );

    // email notify task runner
    let ft_email_notify_task = EmailNotifyTask::new(
        db_client.clone(),
        MailServiceEndpoint::new(task_config.email_domain, task_config.mailgun_auth_token),
    );

    // btc utxo task runner
    let ft_sync_utxos_task = SyncUtxosTask::new(db_client.clone(), bitcoin_rpc.clone());

    // Start both high and low frequency tasks
    task_exec::join_mixed_frequency_tasks!(
        high_freq: task_config.task_interval => [
            ft_sync_balance_task,
            ft_sync_received_payment_task,
            ft_email_notify_task,
            ft_sync_utxos_task
        ],
        low_freq: task_config.task_interval * 10 => [
            ft_sync_price_task,
            ft_sync_currency_task
        ]
    )
    .expect("Task execution failed");
}

struct TaskConfig {
    infura_token: String,
    etherscan_token: String,
    bscscan_token: String,
    mailgun_auth_token: String,
    network_type: NetworkType,
    currency_query_base_url: url::Url,
    currency_query_token: String,
    email_domain: String,
    task_interval: u64,
    db_client_address: url::Url,
    log_path: PathBuf,
}

impl TaskConfig {
    pub fn new(root_path: &Path, config_file_path: &Path) -> Self {
        let runtime_config = RuntimeConfig::from_toml(root_path.join(config_file_path))
            .expect("failed to read runtime config");

        // read from env
        let infura_token = std::env::var("INFURA_TOKEN").expect("INFURA_TOKEN not set");
        let etherscan_token = std::env::var("ETHERSCAN_TOKEN").expect("ETHERSCAN_TOKEN not set");
        let bscscan_token = std::env::var("BSCSCAN_TOKEN").expect("BSCSCAN_TOKEN not set");
        let mailgun_auth_token =
            std::env::var("MAILGUN_AUTH_TOKEN").expect("MAILGUN_AUTH_TOKEN not set");
        let currency_query_token =
            std::env::var("CURRENCY_QUERY_TOKEN").expect("CURRENCY_QUERY_TOKEN not set");

        // read from config
        let network_type = runtime_config.blockchain_network.network_type;
        let currency_query_base_url = runtime_config.blockchain_network.currency_query_base_url;
        let email_domain = runtime_config.task.email_domain;
        let task_interval = runtime_config.task.task_exec_intervals;
        let db_client_address = runtime_config
            .internal_endpoints
            .db_service
            .advertised_address;
        let log_path = root_path.join(runtime_config.storage.log_path);

        Self {
            infura_token,
            etherscan_token,
            bscscan_token,
            mailgun_auth_token,
            network_type,
            currency_query_base_url,
            currency_query_token,
            email_domain,
            task_interval,
            db_client_address,
            log_path,
        }
    }
}
