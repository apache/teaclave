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

use crate::share::AccountId;
use crate::share::{BtcAddress, ClientBtcAddress, EthAddress};
use anyhow::Result;
use bitcoin::bip32::{DerivationPath, Xpub};
use bitcoin::secp256k1::{self, Secp256k1};
use serde::{Deserialize, Serialize};

/// For receiving funds.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountXpub {
    xpub: Xpub,
    path: DerivationPath,
}

impl AccountXpub {
    pub fn new(path: DerivationPath, xpub: Xpub) -> AccountXpub {
        AccountXpub { xpub, path }
    }

    pub fn path(&self) -> &DerivationPath {
        &self.path
    }

    pub fn index(&self) -> u32 {
        *self.path.to_u32_vec().last().unwrap()
    }

    pub fn compute_id(&self) -> AccountId {
        AccountId::from(&self.xpub.public_key)
    }

    fn _derive_pk_from_path<C: secp256k1::Verification>(
        &self,
        path: &DerivationPath,
        secp: &Secp256k1<C>,
    ) -> Result<bitcoin::PublicKey> {
        let xpub = self
            .xpub
            .derive_pub(secp, path)
            .map_err(|e| anyhow::anyhow!("derive pk failed: {}", e))?;
        let pk = bitcoin::PublicKey::new(xpub.public_key);
        Ok(pk)
    }

    pub fn verify_btc_address<C: secp256k1::Verification>(
        &self,
        path: &DerivationPath,
        client_addr: ClientBtcAddress,
        secp: &Secp256k1<C>,
    ) -> Result<BtcAddress> {
        let pk = self._derive_pk_from_path(path, secp)?;
        let recovered_addr = BtcAddress::try_from_pk(pk, path.clone(), self.xpub.network)?;
        anyhow::ensure!(
            recovered_addr.to_string().as_str() == client_addr.address_str(),
            "invalid address"
        );
        Ok(recovered_addr)
    }

    pub fn verify_eth_address<C: secp256k1::Verification>(
        &self,
        _path: &DerivationPath,
        _client_addr: ClientBtcAddress,
        _secp: &Secp256k1<C>,
    ) -> Result<EthAddress> {
        unimplemented!()
    }
}

impl std::ops::Deref for AccountXpub {
    type Target = Xpub;

    fn deref(&self) -> &Self::Target {
        &self.xpub
    }
}
