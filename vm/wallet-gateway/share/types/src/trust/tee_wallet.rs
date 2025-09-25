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

use crate::{
    share::{
        AccountId, AccountXpub, MultiChainAccount, TaApprovalChainBasic, TaOperatorsBasic, WalletID,
    },
    trust::{AccountXpriv, BtcAccountXpriv, EthAccountXpriv, MasterXpriv, WalletSecret},
    Storable,
};
use anyhow::{bail, ensure, Result};
use bitcoin::secp256k1::Secp256k1;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    convert::{TryFrom, TryInto},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeeWallet {
    wallet_id: WalletID, // from external
    master_xpriv: MasterXpriv,
    accounts: HashSet<MultiChainAccount>,
    approval_chain_basic: TaApprovalChainBasic, // from external
    authorized_operators_basic: TaOperatorsBasic, // from external
}

impl Storable<WalletID> for TeeWallet {
    fn unique_id(&self) -> WalletID {
        self.wallet_id.clone()
    }
}

impl TeeWallet {
    pub fn new(
        wallet_id: WalletID,
        approval_chain_basic: TaApprovalChainBasic,
        authorized_operators_basic: TaOperatorsBasic,
        wallet_secret: &WalletSecret,
    ) -> Result<Self> {
        let master_xpriv: MasterXpriv = wallet_secret.try_into()?;
        Ok(Self {
            wallet_id,
            master_xpriv,
            accounts: HashSet::new(),
            approval_chain_basic,
            authorized_operators_basic,
        })
    }

    pub fn id(&self) -> WalletID {
        self.wallet_id.clone()
    }

    pub fn approval_chain_basic(&self) -> &TaApprovalChainBasic {
        &self.approval_chain_basic
    }

    pub fn authorized_operators_basic(&self) -> &TaOperatorsBasic {
        &self.authorized_operators_basic
    }

    pub fn has_account(&self, account_id: &AccountId) -> bool {
        self.accounts.iter().any(|i| &i.id() == account_id)
    }

    pub fn accounts(&self) -> &HashSet<MultiChainAccount> {
        &self.accounts
    }

    pub fn update_approval_chain(
        &mut self,
        approval_chain_basic: TaApprovalChainBasic,
    ) -> Result<()> {
        self.approval_chain_basic = approval_chain_basic;
        Ok(())
    }

    pub fn update_authorized_operators(
        &mut self,
        authorized_operators_basic: TaOperatorsBasic,
    ) -> Result<()> {
        self.authorized_operators_basic = authorized_operators_basic;
        Ok(())
    }

    pub fn add_eth_account(&mut self) -> Result<(EthAccountXpriv, AccountXpub)> {
        let secp = Secp256k1::signing_only();
        let eth_account_xpriv = self.master_xpriv.derive_eth_xpriv(&secp)?;
        let eth_account_xpub = eth_account_xpriv.xpub(&secp);

        self.accounts
            .insert(MultiChainAccount::Eth(eth_account_xpub.clone()));
        Ok((eth_account_xpriv, eth_account_xpub))
    }

    // the network type of child account is the same as the master xpriv,
    // which is defined when master xpriv created.
    pub fn add_btc_account(&mut self) -> Result<(BtcAccountXpriv, AccountXpub)> {
        let secp = Secp256k1::signing_only();
        let btc_account_xpriv = self.master_xpriv.derive_p2wpkh_xpriv(&secp)?;
        let btc_account_xpub = btc_account_xpriv.xpub(&secp);

        self.accounts
            .insert(MultiChainAccount::Btc(btc_account_xpub.clone()));
        Ok((btc_account_xpriv, btc_account_xpub))
    }

    // for restore-wallets: We only backup the TeeWallet, need to derive all AccountXpriv for restore
    pub fn derive_all_xpriv_for_restore(&mut self) -> Result<Vec<AccountXpriv>> {
        let mut result = vec![];
        let secp = Secp256k1::signing_only();
        for multichain_account in self.accounts().clone() {
            if let MultiChainAccount::Eth(account_xpub) = multichain_account {
                let eth_account_xpriv = self
                    .master_xpriv
                    .derive_eth_xpriv_from_path(&secp, account_xpub.path().clone())?;
                result.push(eth_account_xpriv.0);
            } else if let MultiChainAccount::Btc(account_xpub) = multichain_account {
                let btc_account_xpriv = self
                    .master_xpriv
                    .derive_p2wpkh_xpriv_from_path(&secp, account_xpub.path().clone())?;
                result.push(btc_account_xpriv.0);
            } else {
                bail!("invalid account type: {:?}", multichain_account);
            }
        }
        Ok(result)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeeWalletForBackup {
    wallet_id: WalletID,
    master_xpriv: MasterXpriv,
    xpriv_checksum: String,
    accounts: HashSet<MultiChainAccount>,
}

impl TeeWalletForBackup {
    pub fn id(&self) -> WalletID {
        self.wallet_id.clone()
    }

    pub fn xpriv_checksum(&self) -> &str {
        &self.xpriv_checksum
    }
}

impl TryFrom<TeeWallet> for TeeWalletForBackup {
    type Error = anyhow::Error;

    fn try_from(wallet: TeeWallet) -> Result<Self> {
        let xpriv_checksum = wallet.master_xpriv.checksum()?;
        Ok(Self {
            wallet_id: wallet.wallet_id,
            master_xpriv: wallet.master_xpriv,
            xpriv_checksum,
            accounts: wallet.accounts,
        })
    }
}

impl TryFrom<TeeWalletForBackup> for TeeWallet {
    type Error = anyhow::Error;

    fn try_from(wallet: TeeWalletForBackup) -> Result<Self> {
        // check xpriv checksum
        let checksum = wallet.master_xpriv.checksum()?;
        ensure!(
            checksum == wallet.xpriv_checksum,
            "xpriv checksum not match"
        );
        Ok(Self {
            wallet_id: wallet.wallet_id,
            master_xpriv: wallet.master_xpriv,
            accounts: wallet.accounts,
            approval_chain_basic: TaApprovalChainBasic::new(vec![]),
            authorized_operators_basic: TaOperatorsBasic::new(HashSet::new()),
        })
    }
}
