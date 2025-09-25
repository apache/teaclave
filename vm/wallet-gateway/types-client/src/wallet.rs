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

use crate::BalanceForClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use storable::currency::AssetPriceInfo;
use storable::synced_wallet::SyncedWalletInfo;
use types::external::ClientExternalAddress;
use types::external::{
    ApprovalChainBasic, AssetType, CkAccount, CkAmount, OperatorsBasic, ViewersBasic,
};
use types::share::{CkNetwork, NetworkType, WalletID};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WalletInfoForClient {
    wallet_id: WalletID,
    wallet_name: String,
    receive_info: Vec<ClientReceiveInfo>,
    is_subscribed: bool,
    balances: Vec<BalanceForClient>,
    approval_chain: ApprovalChainBasic,
    authorized_operators: OperatorsBasic,
    viewers: ViewersBasic,
}

impl WalletInfoForClient {
    pub fn try_from(
        wallet: SyncedWalletInfo,
        wallet_name: String,
        viewers: ViewersBasic,
        accounts: Vec<CkAccount>,
        balances: Vec<CkAmount>,
        available_balances: HashMap<AssetType, CkAmount>,
        prices: &HashMap<AssetType, AssetPriceInfo>, // we only need to look up the price without taking the ownership
        is_subscribed: bool,
        network_type: NetworkType,
    ) -> Result<Self> {
        let mut balance_with_price = vec![];
        for amount in balances {
            let asset_type = amount.asset_type();
            let price = prices
                .get(&asset_type)
                .ok_or_else(|| anyhow::anyhow!("Price not found"))?;
            let available_balance = available_balances
                .get(&asset_type)
                .copied()
                .unwrap_or(CkAmount::zero(asset_type));
            balance_with_price.push(BalanceForClient::new(
                amount,
                available_balance,
                price.price_in_usd(),
            ));
        }

        let mut receive_info_list = vec![];
        // Generate the invoice address and explorer URL for each account
        // For ETH account, we generate both ETH and BSC receive info
        // For BTC account, we generate only BTC receive info
        for ck_account in accounts {
            match ck_account {
                CkAccount::Eth(_) => {
                    let eth_info =
                        ClientReceiveInfo::try_from(&ck_account, CkNetwork::Eth(network_type))?;
                    receive_info_list.push(eth_info);
                    let bsc_info =
                        ClientReceiveInfo::try_from(&ck_account, CkNetwork::Bsc(network_type))?;
                    receive_info_list.push(bsc_info);
                }
                CkAccount::Btc(_) => {
                    let btc_info =
                        ClientReceiveInfo::try_from(&ck_account, CkNetwork::Btc(network_type))?;
                    receive_info_list.push(btc_info);
                }
            }
        }

        Ok(Self {
            wallet_id: wallet.id,
            wallet_name,
            viewers,
            receive_info: receive_info_list,
            is_subscribed,
            balances: balance_with_price,
            approval_chain: wallet.approval_chain,
            authorized_operators: wallet.authorized_operators,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClientReceiveInfo {
    network_type: CkNetwork,
    asset_type_list: Vec<AssetType>,
    invoice_address: ClientExternalAddress,
    explorer_url: url::Url,
}

impl ClientReceiveInfo {
    pub fn try_from(ck_account: &CkAccount, ck_network: CkNetwork) -> Result<Self> {
        let invoice_address = ck_account.invoice_address_string()?;
        let explorer_url = url::Url::parse(
            format!(
                "{}address/{}",
                ck_network.explorer_base_url(),
                invoice_address
            )
            .as_str(),
        )?;

        Ok(Self {
            network_type: ck_network,
            asset_type_list: AssetType::filter_assets_by_ck_network(&ck_network),
            invoice_address: ClientExternalAddress(invoice_address),
            explorer_url,
        })
    }
}
