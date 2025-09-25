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

use crate::share::{AccountXpub, BtcAddress, ChangeInfo, RecipientInfo, UtxoInfo};
use anyhow::Result;
use bitcoin::{
    bip32::DerivationPath,
    secp256k1::{self, Secp256k1},
    Amount, OutPoint, ScriptBuf, Sequence, TxIn, TxOut, Witness,
};

// CkUtxoInfo can only be generated after validation of UtxoInfo
// 1. address is valid btc_address
// 2. btc_address if derived from account_xpub given the offset
// 3. address matches the script_pubkey in pre_txout
pub struct CkUtxoInfo {
    out_point: OutPoint, // field in cur_tx
    pre_txout: TxOut,    // field in pre_tx
    address: BtcAddress, // addr can spend this utxo
    path: DerivationPath,
}

impl CkUtxoInfo {
    pub fn try_from<C: secp256k1::Verification>(
        xpub: &AccountXpub,
        utxo: UtxoInfo,
        secp: &Secp256k1<C>,
    ) -> Result<CkUtxoInfo> {
        let addr = xpub.verify_btc_address(&utxo.path, utxo.address, secp)?;

        Ok(CkUtxoInfo {
            out_point: utxo.out_point,
            pre_txout: utxo.pre_txout,
            address: addr,
            path: utxo.path,
        })
    }

    pub fn create_input(&self) -> TxIn {
        // The input for the transaction we are constructing.
        TxIn {
            previous_output: self.out_point,  // The dummy output we are spending.
            script_sig: ScriptBuf::default(), // For a p2wpkh script_sig is empty.
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::default(), // Filled in after signing.
        }
    }

    pub fn amount(&self) -> Amount {
        self.pre_txout.value
    }

    pub fn script_pubkey(&self) -> &ScriptBuf {
        &self.pre_txout.script_pubkey
    }

    pub fn address(&self) -> &BtcAddress {
        &self.address
    }

    pub fn path(&self) -> &DerivationPath {
        &self.path
    }
}

// CkRecipientInfo can only be generated after validation of RecipientInfo
pub struct CkRecipientInfo {
    address: bitcoin::Address,
    amount: Amount,
}

impl CkRecipientInfo {
    pub fn create_output(&self) -> TxOut {
        TxOut {
            value: self.amount,
            script_pubkey: self.address.script_pubkey(),
        }
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }
}

impl std::convert::TryFrom<RecipientInfo> for CkRecipientInfo {
    type Error = anyhow::Error;

    fn try_from(info: RecipientInfo) -> Result<Self, Self::Error> {
        let address = bitcoin::Address::try_from(info.address)?;

        Ok(CkRecipientInfo {
            address,
            amount: info.amount,
        })
    }
}

pub struct CkChangeInfo {
    address: BtcAddress,
    path: DerivationPath,
    amount: Amount,
}

impl CkChangeInfo {
    pub fn try_from<C: secp256k1::Verification>(
        xpub: &AccountXpub,
        change: ChangeInfo,
        secp: &Secp256k1<C>,
    ) -> Result<CkChangeInfo> {
        let addr = xpub.verify_btc_address(&change.path, change.address, secp)?;
        Ok(CkChangeInfo {
            address: addr,
            path: change.path,
            amount: change.amount,
        })
    }

    pub fn create_output(&self) -> TxOut {
        TxOut {
            value: self.amount,
            script_pubkey: self.address.script_pubkey(),
        }
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }

    pub fn path(&self) -> &DerivationPath {
        &self.path
    }
}
