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

use crate::share::{AccountXpub, BtcAddress, ClientBtcAddress};
use anyhow::Result;
use bitcoin::bip32::DerivationPath;
use bitcoin::secp256k1::{self, Secp256k1};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BtcAccount {
    account_xpub: AccountXpub,
    invoice_derived: u32,
    invoice_addresses: HashSet<ClientBtcAddress>, // task runner needs to get all addresses periodically, store them for later use
    change_derived: u32,
    change_addresses: HashSet<ClientBtcAddress>,
}

/// According to BIP-48:
/// Currently the only script types covered by this BIP are Native Segwit (p2wsh) and Nested Segwit (p2sh-p2wsh).
/// The following path represents Nested Segwit (p2sh-p2wsh) mainnet, account 0: 1': Nested Segwit (p2sh-p2wsh) m/48'/0'/0'/1'
/// The following path represents Native Segwit (p2wsh) mainnet, account 0: 2': Native Segwit (p2wsh) m/48'/0'/0'/2'
/// The recommended default for wallets is pay to witness script hash m/48'/0'/0'/2'.
///
/// According to P2WPKH: BIP 84
/// [m / purpose' / coin_type' / account' /] / change / address_index
pub(crate) fn sub_address_derive_path(
    is_change: bool,
    address_index: u32,
) -> Result<DerivationPath> {
    let change: u32 = match is_change {
        true => 1,
        false => 0,
    };
    let sub_path = format!("m/{}/{}", change, address_index);
    DerivationPath::from_str(&sub_path).map_err(|_| anyhow::anyhow!("invalid derivation path"))
}

impl BtcAccount {
    pub fn try_init(account_xpub: AccountXpub) -> Result<BtcAccount> {
        let mut account = BtcAccount {
            account_xpub,
            invoice_derived: 0,
            change_derived: 0,
            invoice_addresses: HashSet::new(),
            change_addresses: HashSet::new(),
        };
        let secp = Secp256k1::verification_only();
        account.derive_invoice_address(&secp)?;
        account.derive_change_address(&secp)?;
        Ok(account)
    }

    fn _derive_address<C: secp256k1::Verification>(
        &self,
        is_change: bool,
        index: u32,
        secp: &Secp256k1<C>,
    ) -> Result<BtcAddress> {
        let sub_path = sub_address_derive_path(is_change, index)?;
        let xpub = self
            .account_xpub
            .derive_pub(secp, &sub_path)
            .map_err(|e| anyhow::anyhow!("derive address failed: {}", e))?;
        let pk = bitcoin::PublicKey::new(xpub.public_key);
        BtcAddress::try_from_pk(pk, sub_path, xpub.network)
    }

    pub fn derive_invoice_address<C: secp256k1::Verification>(
        &mut self,
        secp: &Secp256k1<C>,
    ) -> Result<BtcAddress> {
        let index = self.invoice_derived;
        let addr = self._derive_address(false, index, secp)?;
        self.invoice_derived += 1;
        self.invoice_addresses.insert((&addr).into());
        Ok(addr)
    }

    pub fn current_invoice_address(&self) -> Result<BtcAddress> {
        let secp = Secp256k1::verification_only();
        anyhow::ensure!(self.invoice_derived > 0, "no invoice address derived");
        let index = self.invoice_derived - 1;
        let addr = self._derive_address(false, index, &secp)?;
        Ok(addr)
    }

    pub fn current_change_address(&self) -> Result<BtcAddress> {
        let secp = Secp256k1::verification_only();
        anyhow::ensure!(self.change_derived > 0, "no change address derived");
        let index = self.change_derived - 1;
        let addr = self._derive_address(false, index, &secp)?;
        Ok(addr)
    }

    pub fn derive_change_address<C: secp256k1::Verification>(
        &mut self,
        secp: &Secp256k1<C>,
    ) -> Result<BtcAddress> {
        let index = self.change_derived;
        let addr = self._derive_address(true, index, secp)?;
        self.change_derived += 1;
        self.change_addresses.insert((&addr).into());
        Ok(addr)
    }

    pub fn invoice_address(&self) -> Result<BtcAddress> {
        self.current_invoice_address()
    }

    pub fn id(&self) -> crate::share::AccountId {
        self.compute_id()
    }

    pub fn all_client_addresses(&self) -> HashSet<ClientBtcAddress> {
        let mut addresses = self.all_client_invoice_addresses();
        addresses.extend(self.all_client_change_addresses());
        addresses
    }

    pub fn all_client_invoice_addresses(&self) -> HashSet<ClientBtcAddress> {
        self.invoice_addresses.clone().into_iter().collect()
    }

    pub fn all_client_change_addresses(&self) -> HashSet<ClientBtcAddress> {
        self.change_addresses.clone().into_iter().collect()
    }

    pub fn all_addresses(&self) -> Result<Vec<BtcAddress>> {
        let mut addresses = Vec::new();
        // derive all invoice addresses
        for i in 0..self.invoice_derived {
            let addr = self._derive_address(false, i, &Secp256k1::verification_only())?;
            addresses.push(addr);
        }
        // derive all change addresses
        for i in 0..self.change_derived {
            let addr = self._derive_address(true, i, &Secp256k1::verification_only())?;
            addresses.push(addr);
        }
        Ok(addresses)
    }
}

impl std::ops::Deref for BtcAccount {
    type Target = AccountXpub;

    fn deref(&self) -> &Self::Target {
        &self.account_xpub
    }
}
