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

use crate::share::{AccountId, BtcAddress, CkHash, CkHasher, ClientBtcAddress, WalletID};
use anyhow::Result;
use bitcoin::{bip32::DerivationPath, Amount, OutPoint, TxOut};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BtcTransaction {
    pub from_wallet: WalletID,
    pub from_account: AccountId,
    pub utxo_list: Vec<UtxoInfo>,
    pub recipient_list: Vec<RecipientInfo>,
    pub change: ChangeInfo,
}

impl BtcTransaction {
    pub fn from_account(&self) -> &AccountId {
        &self.from_account
    }

    pub fn from_wallet(&self) -> &WalletID {
        &self.from_wallet
    }
}

impl CkHasher for BtcTransaction {
    fn hash(&self) -> Result<CkHash> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.from_account.as_bytes());
        bytes.extend_from_slice(bincode::serialize(&self.utxo_list)?.as_slice());
        bytes.extend_from_slice(bincode::serialize(&self.recipient_list)?.as_slice());
        bytes.extend_from_slice(bincode::serialize(&self.change)?.as_slice());

        CkHash::new(bytes)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UtxoInfo {
    pub out_point: OutPoint,       // field in cur_tx
    pub pre_txout: TxOut,          // field in pre_tx
    pub address: ClientBtcAddress, // addr can spend this utxo
    pub path: DerivationPath,      // address offset (this must be a ckaddress)
                                   // can be change or , address
}

impl UtxoInfo {
    pub fn new(
        out_point: OutPoint,
        pre_txout: TxOut,
        address: ClientBtcAddress,
        path: DerivationPath,
    ) -> UtxoInfo {
        UtxoInfo {
            out_point,
            pre_txout,
            address,
            path,
        }
    }

    pub fn value(&self) -> Amount {
        self.pre_txout.value
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RecipientInfo {
    pub address: ClientBtcAddress,
    pub amount: Amount,
}

impl RecipientInfo {
    pub fn new(address: ClientBtcAddress, amount: Amount) -> RecipientInfo {
        RecipientInfo { address, amount }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ChangeInfo {
    pub address: ClientBtcAddress,
    pub path: DerivationPath, // address offset, must be a ckaddress
    pub amount: Amount,
}

impl ChangeInfo {
    pub fn new(addr: BtcAddress, amount: Amount) -> ChangeInfo {
        let path = addr.path().clone();
        ChangeInfo {
            address: addr.into(),
            path,
            amount,
        }
    }
}
