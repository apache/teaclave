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

use std::convert::TryInto;

use anyhow::Result;
use bdk_coin_select::{Candidate, Drain, DrainWeights, FeeRate, Target};
use bitcoin::{Amount, TxOut};
use types::share::{BtcAddress, ChangeInfo, RecipientInfo, UtxoInfo};

const ESTIMATED_P2WPKH_INPUT_WEIGHT: u32 = 68 * 4;
const MAX_SELECTION_ROUND: usize = 100_000;

#[derive(Default)]
pub enum ChangePolicy {
    MinValue,
    #[default]
    MinValueAndWaste,
}

#[derive(Default)]
pub enum CoinSelectionStrategy {
    #[default]
    LowestFee,
    Changeless,
}

pub struct CoinSelector {
    pub strategy: CoinSelectionStrategy,
    pub change_policy: ChangePolicy,
    pub total_spend_amount: Amount, // sum(recipients.amount)
    pub base_tx: bitcoin::Transaction,
    pub base_tx_weight: u32,
    pub utxo_list: Vec<UtxoInfo>,
    pub candidates: Vec<Candidate>,
}

impl CoinSelector {
    pub fn new(
        strategy: CoinSelectionStrategy,
        change_policy: ChangePolicy,
        utxos: Vec<UtxoInfo>,
        receipients: Vec<RecipientInfo>,
    ) -> Result<Self> {
        let base_tx = Self::construct_base_tx(&receipients)?;
        let base_tx_weight = base_tx.weight().to_wu() as u32;
        log::info!("base_tx_weight: {}", base_tx_weight);

        let candidates = utxos
            .iter()
            .map(|utxo| Candidate {
                input_count: 1,
                value: utxo.pre_txout.value.to_sat(),
                weight: ESTIMATED_P2WPKH_INPUT_WEIGHT,
                is_segwit: utxo.pre_txout.script_pubkey.is_witness_program(),
            })
            .collect::<Vec<Candidate>>();

        let total_spend_amount = receipients.into_iter().fold(Amount::ZERO, |acc, r| {
            r.amount.checked_add(acc).unwrap_or_default()
        });

        Ok(Self {
            strategy,
            change_policy,
            total_spend_amount,
            base_tx,
            base_tx_weight,
            utxo_list: utxos,
            candidates,
        })
    }

    pub fn select(
        &self,
        fee_rate: f64,
        change_address: BtcAddress,
    ) -> Result<(Vec<UtxoInfo>, ChangeInfo, u32)> {
        // u32: total weight
        let mut selector =
            bdk_coin_select::CoinSelector::new(&self.candidates, self.base_tx_weight);

        let target = Target {
            value: self.total_spend_amount.to_sat(),
            feerate: FeeRate::from_sat_per_vb(fee_rate as f32),
            min_fee: 0,
        };

        let drain_weights = self.calculate_drain_weights(&change_address)?;
        let dust = change_address.script_pubkey().dust_value().to_sat();
        let change_policy = match self.change_policy {
            ChangePolicy::MinValue => {
                bdk_coin_select::ChangePolicy::min_value(
                    drain_weights,
                    dust, // min dust value: 294
                )
            }
            ChangePolicy::MinValueAndWaste => {
                bdk_coin_select::ChangePolicy::min_value_and_waste(
                    drain_weights,
                    dust,           // min dust value: 294
                    target.feerate, // current fee rate
                    target.feerate, // longterm fee rate
                )
            }
        };

        match self.strategy {
            CoinSelectionStrategy::LowestFee => {
                let metric = bdk_coin_select::metrics::LowestFee {
                    target,
                    long_term_feerate: target.feerate,
                    change_policy,
                };
                self.run_selection(&mut selector, metric, target)?;
            }
            CoinSelectionStrategy::Changeless => {
                let metric = bdk_coin_select::metrics::Changeless {
                    target,
                    change_policy,
                };
                self.run_selection(&mut selector, metric, target)?;
            }
        };

        let selection = selector
            .apply_selection(&self.candidates)
            .collect::<Vec<_>>();
        let change = selector.drain(target, change_policy);

        log::info!(
            "we selected {} inputs, weight: {}",
            selection.len(),
            selector.input_weight()
        );
        log::info!(
            "We are including a change output of {} value (0 means not change)",
            change.value
        );

        // filter the result
        let utxo_info_results: Vec<UtxoInfo> =
            selector.apply_selection(&self.utxo_list).cloned().collect();
        log::debug!("utxo_info_results: {:?}", utxo_info_results);

        // convert to ChangeInfo
        let change_info = ChangeInfo::new(change_address, Amount::from_sat(change.value));

        Ok((
            utxo_info_results,
            change_info,
            selector.weight(change.weights.output_weight),
        ))
    }

    fn run_selection<M>(
        &self,
        selector: &mut bdk_coin_select::CoinSelector,

        metric: M,
        target: Target,
    ) -> Result<()>
    where
        M: bdk_coin_select::BnbMetric,
    {
        match selector.run_bnb(metric, MAX_SELECTION_ROUND) {
            Err(err) => {
                log::info!("failed to find a solution: {}", err);
                // if cannot find a solution, use the first solution that meets the target
                selector
                    .select_until_target_met(target, Drain::none())
                    .map_err(|e| anyhow::anyhow!(e))
            }
            Ok(score) => {
                log::info!("we found a solution with score {}", score);
                Ok(())
            }
        }
    }

    fn construct_base_tx(receipients: &Vec<RecipientInfo>) -> Result<bitcoin::Transaction> {
        let mut base_tx = bitcoin::Transaction {
            version: bitcoin::blockdata::transaction::Version(2),
            lock_time: bitcoin::absolute::LockTime::from_height(0).unwrap(),
            input: vec![],
            output: vec![],
        };
        for receipient in receipients {
            let addr: bitcoin::Address = receipient.address.clone().try_into()?;
            let output = bitcoin::TxOut {
                value: receipient.amount,
                script_pubkey: addr.script_pubkey(),
            };
            base_tx.output.push(output);
        }
        Ok(base_tx)
    }

    fn calculate_drain_weights(&self, change_address: &BtcAddress) -> Result<DrainWeights> {
        let mut tx_with_drain = self.base_tx.clone();
        tx_with_drain
            .output
            .push(TxOut::minimal_non_dust(change_address.script_pubkey()));
        let drain_output_weight = tx_with_drain.weight().to_wu() as u32 - self.base_tx_weight;

        let drain_weights = DrainWeights {
            output_weight: drain_output_weight,
            spend_weight: ESTIMATED_P2WPKH_INPUT_WEIGHT,
        };

        Ok(drain_weights)
    }

    fn calculate_drain_cost(&self, drain_weights: DrainWeights, fee_rate: FeeRate) -> Result<f32> {
        // the first fee_rate: the change as output
        // the second fee_rate: the change as input further (longterm feerate)
        let drain_cost = drain_weights.waste(fee_rate, fee_rate);
        log::info!(
            "drain sum weights: {:?}. drain cost: {}",
            drain_weights,
            drain_cost
        );
        Ok(drain_cost)
    }
}
