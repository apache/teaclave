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

use crate::{share::WalletID, trust::MasterXpriv, Storable};
use anyhow::{anyhow, Result};
use basic_utils::generate_random_bytes;
use bip39::Mnemonic;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletSecret {
    wallet_id: WalletID, // as storage key
    entropy: Vec<u8>,
}

impl Storable<WalletID> for WalletSecret {
    fn unique_id(&self) -> WalletID {
        self.wallet_id.clone()
    }
}

impl WalletSecret {
    pub fn new(wallet_id: WalletID) -> Result<Self> {
        Ok(Self {
            wallet_id,
            entropy: generate_random_bytes(32)?,
        })
    }
}

impl TryFrom<&WalletSecret> for MasterXpriv {
    type Error = anyhow::Error;

    fn try_from(wallet_secret: &WalletSecret) -> Result<MasterXpriv> {
        let mnemonic = Mnemonic::from_entropy(&wallet_secret.entropy)
            .map_err(|_| anyhow!("[-] WalletSecret::new(): invalid entropy"))?;
        let seed = mnemonic.to_seed_normalized("CK");
        let master_xpriv = MasterXpriv::from_seed(&seed)?;
        Ok(master_xpriv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_mnemonic() {
        let entropy_test_vector =
            hex::decode("7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f")
                .unwrap();
        let mnemonic_phrase_test_vector = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title"
            .to_string();
        let seed_test_vector = hex::decode("761914478ebf6fe16185749372e91549361af22b386de46322cf8b1ba7e92e80c4af05196f742be1e63aab603899842ddadf4e7248d8e43870a4b6ff9bf16324")
            .unwrap();
        let mnemonic = Mnemonic::from_entropy(&entropy_test_vector).unwrap();
        assert!(mnemonic_phrase_test_vector == format!("{}", mnemonic));
        let seed = mnemonic.to_seed_normalized("");
        assert!(seed_test_vector == seed);
    }
}
