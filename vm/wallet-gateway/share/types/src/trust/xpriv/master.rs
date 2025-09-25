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

use crate::trust::{BtcAccountXpriv, EthAccountXpriv, BTC_NETWORK};
use crate::{
    share::{AccountId, AccountXpub},
    Storable,
};
use anyhow::Result;
use basic_utils::keccak_hash_to_bytes;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::secp256k1::{self, Secp256k1};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Seed -> MasterXpriv
/// MassterXpriv can derive AccountXpriv based on different BIPs
/// For each BIP, we keep record of total number of accounts derived from it
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasterXpriv {
    xpriv: Xpriv,
    bip44_derived: u32, // eth
    bip84_derived: u32, // p2wpkh
}

impl MasterXpriv {
    // m / purpose' / coin_type' / account' [/ script_type' / change / address_index ]
    pub fn from_seed(seed: &[u8]) -> Result<MasterXpriv> {
        let master = Xpriv::new_master(BTC_NETWORK, seed).map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(MasterXpriv {
            xpriv: master,
            bip44_derived: 0,
            bip84_derived: 0,
        })
    }

    fn derive_account_xpriv<C: secp256k1::Signing>(
        &mut self,
        path: DerivationPath,
        secp: &Secp256k1<C>,
    ) -> Result<AccountXpriv> {
        let xpriv = self
            .xpriv
            .derive_priv(secp, &path)
            .map_err(|e| anyhow::anyhow!("failed to derive account xpriv: {}", e))?;
        Ok(AccountXpriv::new(path, xpriv, secp))
    }

    /// BIP-84 specifies that the derivation path for P2WPKH (We would like to support)
    /// BIP-44 specifies that the derivation path for P2PKH
    /// BIP-48 specifies that the derivation path for P2WSH (Multi-Sig Wallets)
    /// BIP-49 specifies that the derivation path for P2SH-P2WPKH (P2WPKH-nested-in-P2SH based accounts)
    /// coin_type is specified in BIP-44
    pub fn derive_p2wpkh_xpriv<C: secp256k1::Signing>(
        &mut self,
        secp: &Secp256k1<C>,
    ) -> Result<BtcAccountXpriv> {
        let account_index = self.bip84_derived;
        let path = bip84_account_path(BTC_NETWORK, account_index)?;
        let account_xpriv = self.derive_account_xpriv(path, secp)?;
        Ok(BtcAccountXpriv(account_xpriv))
    }

    pub fn derive_p2wpkh_xpriv_from_path<C: secp256k1::Signing>(
        &mut self,
        secp: &Secp256k1<C>,
        path: DerivationPath,
    ) -> Result<BtcAccountXpriv> {
        let account_xpriv = self.derive_account_xpriv(path, secp)?;
        Ok(BtcAccountXpriv(account_xpriv))
    }

    pub fn derive_eth_xpriv<C: secp256k1::Signing>(
        &mut self,
        secp: &Secp256k1<C>,
    ) -> Result<EthAccountXpriv> {
        let account_index = self.bip44_derived;
        let path = eth_account_path(account_index)?;
        let account_xpriv = self.derive_account_xpriv(path, secp)?;
        Ok(EthAccountXpriv(account_xpriv))
    }

    pub fn derive_eth_xpriv_from_path<C: secp256k1::Signing>(
        &mut self,
        secp: &Secp256k1<C>,
        path: DerivationPath,
    ) -> Result<EthAccountXpriv> {
        let account_xpriv = self.derive_account_xpriv(path, secp)?;
        Ok(EthAccountXpriv(account_xpriv))
    }

    pub fn checksum(&self) -> Result<String> {
        // calculate hash of xpriv
        let hash = keccak_hash_to_bytes(&bincode::serialize(&self)?);
        Ok(hex::encode(hash[0..16].to_vec()))
    }
}

pub(crate) fn bip84_account_path(
    network: bitcoin::Network,
    account_index: u32,
) -> Result<DerivationPath> {
    let coin_type = match network {
        bitcoin::Network::Bitcoin => 0,
        bitcoin::Network::Testnet => 1,
        _ => return Err(anyhow::anyhow!("invalid network for bip84")),
    };
    let path = format!("m/84'/{}'/{}'", coin_type, account_index);
    DerivationPath::from_str(&path).map_err(|_| anyhow::anyhow!("invalid derivation path"))
}

pub(crate) fn eth_account_path(account_index: u32) -> Result<DerivationPath> {
    // BIP44: m / purpose' / coin_type' / account' / change / address_index
    // registered coin_type for ETH is 60, ref: https://github.com/satoshilabs/slips/blob/master/slip-0044.md
    // For the derivation path, wallets on the market use different implementations
    // ref: https://github.com/ethereum/EIPs/issues/84#issuecomment-292324521
    // we choose the path: m/44'/60'/0'/0/x for compatibility with old version
    let path = format!("m/44'/60'/0'/0/{}", account_index);
    DerivationPath::from_str(&path).map_err(|_| anyhow::anyhow!("invalid derivation path"))
}

/// For spending funds.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountXpriv {
    account_id: AccountId, // as storage key
    path: DerivationPath,
    xpriv: Xpriv,
}

impl Storable<AccountId> for AccountXpriv {
    fn unique_id(&self) -> AccountId {
        self.account_id.clone()
    }
}

impl AccountXpriv {
    pub fn new<C: secp256k1::Signing>(
        path: DerivationPath,
        xpriv: Xpriv,
        secp: &Secp256k1<C>,
    ) -> AccountXpriv {
        let xpub = Xpub::from_priv(secp, &xpriv);
        let account_id = AccountId::from(&xpub.public_key);
        AccountXpriv {
            account_id,
            path,
            xpriv,
        }
    }

    pub fn xpub<C: secp256k1::Signing>(&self, secp: &Secp256k1<C>) -> AccountXpub {
        let xpub = Xpub::from_priv(secp, &self.xpriv);
        AccountXpub::new(self.path.clone(), xpub)
    }
}

impl std::ops::Deref for AccountXpriv {
    type Target = Xpriv;

    fn deref(&self) -> &Self::Target {
        &self.xpriv
    }
}
