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

use crate::share::{BtcAddress, CkNetwork, CkSignature};
use crate::trust::{AccountXpriv, CkChangeInfo, CkRecipientInfo, CkUtxoInfo};
use anyhow::Result;
use bitcoin::bip32::DerivationPath;
use bitcoin::locktime::absolute;
use bitcoin::secp256k1::{self, Message, Secp256k1};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{transaction, Amount, SegwitV0Sighash, Transaction, TxIn, TxOut, Witness};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BtcAccountXpriv(pub AccountXpriv);

impl BtcAccountXpriv {
    pub fn sign_tx<C: secp256k1::Signing>(
        &self,
        utxos: Vec<CkUtxoInfo>,
        recipients: Vec<CkRecipientInfo>,
        change: CkChangeInfo,
        secp: &Secp256k1<C>,
    ) -> Result<CkSignature> {
        // serialize the signed tx
        // The input for the transaction we are constructing.
        let inputs = utxos
            .iter()
            .map(|utxo| utxo.create_input())
            .collect::<Vec<TxIn>>();

        // The spend output is locked to a key controlled by the receiver.
        let mut outputs: Vec<TxOut> = recipients
            .iter()
            .map(|recipient| recipient.create_output())
            .collect::<Vec<TxOut>>();

        let total_amount = utxos
            .iter()
            .fold(Amount::ZERO, |acc, utxo| acc + utxo.amount());
        let total_spent: Amount = recipients
            .iter()
            .fold(Amount::ZERO, |acc, recipient| acc + recipient.amount());
        let left_amount = total_amount.checked_sub(total_spent).ok_or_else(|| {
            anyhow::anyhow!("invalid left amount {} - {}", total_amount, total_spent)
        })?;
        println!("left amount: {}", left_amount);

        // The change output is locked to a key controlled by us.
        // if change amount is zero, we don't create a change output
        if change.amount() > Amount::ZERO {
            println!("change amount: {}", change.amount());
            let out_change = change.create_output();
            outputs.push(out_change);
        }

        // The transaction we want to sign and broadcast.
        let unsigned_tx = Transaction {
            version: transaction::Version::TWO,  // Post BIP-68.
            lock_time: absolute::LockTime::ZERO, // Ignore the locktime.
            input: inputs,
            output: outputs, // Outputs, order does not matter.
        };

        let sighash_type = EcdsaSighashType::All;
        let mut sighasher = SighashCache::new(unsigned_tx);

        for (i, utxo) in utxos.iter().enumerate() {
            // Get the sighash to sign.
            let sighash = sighasher
                .p2wpkh_signature_hash(
                    i,                    // calculate sighash for which txin => hash(outpoint)
                    utxo.script_pubkey(), // what is the script_pubkey of the outpoint (who can spend/redeem) => hash(pre_tx_out)
                    utxo.amount(), // how much to spend (we exaust each utxo) => hash(spend_amount)
                    sighash_type,
                )
                .map_err(|e| anyhow::anyhow!("sighash failed: {}", e))?;
            // Sign the sighash using the secp256k1 library (exported by rust-bitcoin).
            let witness =
                self.create_witness(utxo.address(), utxo.path(), sighash, sighash_type, secp)?;
            // Update the witness stack.
            let ith_wit = sighasher
                .witness_mut(i)
                .ok_or_else(|| anyhow::anyhow!("invalid witness index"))?;
            *ith_wit = witness;
        }

        // Get the signed transaction.
        let tx = sighasher.into_transaction();
        let raw_tx = bitcoin::consensus::serialize(&tx);

        Ok(CkSignature::new(raw_tx))
    }

    fn derive_signing_key<C: secp256k1::Signing>(
        &self,
        addr: &BtcAddress,
        path: &DerivationPath,
        secp: &Secp256k1<C>,
    ) -> Result<bitcoin::secp256k1::SecretKey> {
        let derived_xpriv = self
            .0
            .derive_priv(secp, path)
            .map_err(|e| anyhow::anyhow!("derive signing key failed: {}", e))?;
        let sk = derived_xpriv.to_priv();
        let pk = sk.public_key(secp);
        let derived_addr =
            BtcAddress::try_from_pk(pk, path.clone(), CkNetwork::Btc(addr.network()).try_into()?)?;
        anyhow::ensure!(
            &derived_addr == addr,
            "invalid address to derive signing key"
        );
        let ecdsa_sk = derived_xpriv.private_key;
        Ok(ecdsa_sk)
    }

    fn create_witness<C: secp256k1::Signing>(
        &self,
        address: &BtcAddress,
        path: &DerivationPath,
        sighash: SegwitV0Sighash,
        hash_ty: EcdsaSighashType,
        secp: &Secp256k1<C>,
    ) -> Result<Witness> {
        let msg = Message::from(sighash);

        let sk = self.derive_signing_key(address, path, secp)?;
        let sig = secp.sign_ecdsa(&msg, &sk);
        let signature = bitcoin::ecdsa::Signature { sig, hash_ty };

        let pk = sk.public_key(secp);
        let witness = Witness::p2wpkh(&signature, &pk);
        Ok(witness)
    }
}

impl std::ops::Deref for BtcAccountXpriv {
    type Target = AccountXpriv;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
