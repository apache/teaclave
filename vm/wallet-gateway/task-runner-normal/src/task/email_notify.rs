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

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use db_manager::StorageClient;
use net::MailServiceEndpoint;
use notification::{
    email_event::{NotifyEvent, NotifyEventInfo},
    email_task::SendEmailTask,
    email_template::{
        EmailRequest, EmailTemplate, EmailTemplateCancelled, EmailTemplateDeclined,
        EmailTemplateNeedReSign, EmailTemplateOnChain, EmailTemplatePendingForApproval,
        EmailTemplateReadyForSigning, EmailTemplateReceived, EmailTemplateRejectedByNetwork,
        EmailTemplateTxCreated, TxBasicInfoEmailRequest, TxCompletedEmailRequest,
        TxNetworkEmailRequest, TxReceivedEmailRequest, TxRequiredApprovalEmailRequest,
    },
};
use std::{collections::HashSet, ops::Deref, sync::Arc};
use storable::{
    address_book::AddressBookEntry,
    console::ConsoleWalletInfo,
    nickname::NicknameValue,
    pending_tx::PendingTx,
    received_payment::PaymentRecord,
    subscription::SubscriptionInfo,
    synced_wallet::SyncedWalletInfo,
    tx_history::{HistoryTx, TxHistoryStatus},
};
use task_exec::Executable;
use types::{
    external::Email,
    share::{NetworkType, WalletID},
};

macro_rules! process_email_template {
    ($self:expr, $template:expr, $recipients:expr, $wallet_id:expr, $is_received_payment:expr) => {{
        let template = Box::new($template);
        let subscribed_recipients = $self
            .get_subscribed_recipients($recipients, $wallet_id)
            .await?;
        $self
            .finalize_email_task(
                subscribed_recipients,
                template,
                $is_received_payment,
                $wallet_id,
            )
            .await
    }};
}

pub struct EmailNotifyTask {
    db_client: Arc<StorageClient>,
    endpoint: MailServiceEndpoint,
}
impl EmailNotifyTask {
    pub fn new(db_client: Arc<StorageClient>, endpoint: MailServiceEndpoint) -> Self {
        Self {
            db_client,
            endpoint,
        }
    }

    async fn inner_exec(&self) -> Result<()> {
        // fetch all notify event info
        for (event_id, notify_event_info) in
            self.db_client.list_entries::<_, NotifyEventInfo>().await?
        {
            // add nickname into each recepient's email content
            let send_email_tasks = match notify_event_info.event {
                NotifyEvent::HistoryEvent(tx_history) => {
                    self.process_history_tx(tx_history).await?
                }
                NotifyEvent::PendingEvent(pending_tx) => {
                    self.process_pending_tx(pending_tx).await?
                }
                NotifyEvent::ReceivedPayment(payment_record) => {
                    self.process_received_payment(payment_record).await?
                }
            };

            // send email
            match self.endpoint.send_email(&send_email_tasks).await {
                Ok(_) => {
                    // remove notify event info
                    self.db_client
                        .delete_entry::<_, NotifyEventInfo>(&event_id)
                        .await?;
                }
                Err(e) => {
                    log::error!("failed to send email: {:?}", e);
                }
            }
        }
        Ok(())
    }

    async fn process_history_tx(&self, tx_history: HistoryTx) -> Result<Vec<SendEmailTask>> {
        let tx_transfer = tx_history.tx().transfer_info();

        // common info
        let from_wallet = tx_transfer.from_wallet();
        let to = tx_transfer.recipient_string();
        let amount = *tx_transfer.amount();
        let tx_id: String = tx_history.tx().get_id().into();

        // tx_query_url is used for "show tx" button in email
        let domain_url = url::Url::parse("https://ck.com")?;
        let tx_query_url = domain_url.join(&format!("transactions/{}", tx_id))?;

        // the template param name is etherscan_link (legacy prod), but it's used for both btc and eth
        let etherscan_base_url = tx_transfer
            .amount
            .asset_type()
            .as_ck_network(NetworkType::Mainnet)
            .explorer_base_url();

        // send email to the operator who created the tx
        let operator: Email = tx_history.tx().get_operator().clone();
        // send email to all related approvers
        let mut recipients: HashSet<Email> = tx_history.tx().get_all_approvers();
        recipients.insert(operator);
        // send email to all viewers
        let viewers = self.get_viewers_for_wallet(&from_wallet).await?;
        recipients.extend(viewers);

        match tx_history.tx_status() {
            TxHistoryStatus::OnChain(tx_hash) => {
                process_email_template!(
                    self,
                    EmailTemplateOnChain(TxCompletedEmailRequest {
                        from: from_wallet.clone().into(),
                        to,
                        amount: amount.try_to_f64()?,
                        asset: amount.asset_type(),
                        etherscan_link: etherscan_base_url.join(&format!("tx/{:?}", tx_hash))?,
                        tx_link: tx_query_url,
                    }),
                    &recipients,
                    &from_wallet,
                    false
                )
            }
            TxHistoryStatus::RejectedByNetwork(_error_info) => {
                process_email_template!(
                    self,
                    EmailTemplateRejectedByNetwork(TxNetworkEmailRequest {
                        from: from_wallet.clone().into(),
                        to,
                        amount: amount.try_to_f64()?,
                        asset: amount.asset_type(),
                        message: tx_history.message(),
                        tx_link: tx_query_url,
                    }),
                    &recipients,
                    &from_wallet,
                    false
                )
            }
            TxHistoryStatus::RejectedByApprover => {
                process_email_template!(
                    self,
                    EmailTemplateDeclined(TxBasicInfoEmailRequest {
                        from: from_wallet.clone().into(),
                        to,
                        amount: amount.try_to_f64()?,
                        asset: amount.asset_type(),
                        tx_link: tx_query_url,
                    }),
                    &recipients,
                    &from_wallet,
                    false
                )
            }
            TxHistoryStatus::Cached => {
                process_email_template!(
                    self,
                    EmailTemplateRejectedByNetwork(TxNetworkEmailRequest {
                        from: from_wallet.clone().into(),
                        to,
                        amount: amount.try_to_f64()?,
                        asset: amount.asset_type(),
                        message: tx_history.message(),
                        tx_link: tx_query_url,
                    }),
                    &recipients,
                    &from_wallet,
                    false
                )
            }
            TxHistoryStatus::RecalledByOperator => {
                process_email_template!(
                    self,
                    EmailTemplateCancelled(TxBasicInfoEmailRequest {
                        from: from_wallet.clone().into(),
                        to,
                        amount: amount.try_to_f64()?,
                        asset: amount.asset_type(),
                        tx_link: tx_query_url,
                    }),
                    &recipients,
                    &from_wallet,
                    false
                )
            }
        }
    }

    async fn process_received_payment(
        &self,
        payment_record: PaymentRecord,
    ) -> Result<Vec<SendEmailTask>> {
        let transfer_info = payment_record.transfer_info;
        let explorer_base_url = transfer_info
            .amount
            .asset_type()
            .as_ck_network(NetworkType::Mainnet)
            .explorer_base_url();

        let to_wallet = transfer_info.to_wallet;
        let mut associated_users = self.get_associated_users(&to_wallet).await?;
        // send email to all viewers
        let viewers = self.get_viewers_for_wallet(&to_wallet).await?;
        associated_users.extend(viewers);

        process_email_template!(
            self,
            EmailTemplateReceived(TxReceivedEmailRequest {
                from: transfer_info.from.to_string(),
                to: to_wallet.as_string(),
                amount: transfer_info.amount.try_to_f64()?,
                asset: transfer_info.amount.asset_type(),
                etherscan_link: explorer_base_url
                    .join(&format!("tx/{}", payment_record.tx_hash))?,
            }),
            &associated_users,
            &to_wallet,
            true
        )
    }

    async fn process_pending_tx(&self, pending_tx: PendingTx) -> Result<Vec<SendEmailTask>> {
        let tx_transfer = pending_tx.tx().transfer_info();
        // common info
        let from_wallet = tx_transfer.from_wallet();
        let to = tx_transfer.recipient_string();
        let amount = *tx_transfer.amount();
        let tx_id: String = pending_tx.tx().get_id().into();

        // tx_query_url is used for "show tx" button in email
        let domain_url = url::Url::parse("https://ck.com")?;
        let tx_query_url = domain_url.join(&format!("transactions/{}", tx_id))?;

        // send email to the operator who created the tx
        let operators = HashSet::from_iter(vec![pending_tx.tx().get_operator().clone()]);
        // send email to viewers
        let viewers = self.get_viewers_for_wallet(&from_wallet).await?;
        let mut email_tasks = Vec::new();

        match pending_tx {
            PendingTx::PendingForApproval(tx) => {
                // if in stage 0, send to viewers
                let tasks = if tx.current_stage_index().ok_or(anyhow!(
                    "current stage not found for tx: {:?}, status error",
                    &tx_id
                ))? == 0
                {
                    process_email_template!(
                        self,
                        EmailTemplateTxCreated(TxBasicInfoEmailRequest {
                            from: from_wallet.clone().into(),
                            to: to.clone(),
                            amount: amount.try_to_f64()?,
                            asset: amount.asset_type(),
                            tx_link: tx_query_url.clone(),
                        }),
                        &viewers,
                        &from_wallet,
                        false
                    )?
                } else {
                    Vec::new()
                };
                email_tasks.extend(tasks);

                // current pending stage approvers
                let current_pending_stage_approvers: HashSet<Email> =
                    tx.current_stage_approvers().ok_or_else(|| {
                        anyhow!(
                            "next stage approvers not found for tx: {:?}, status error",
                            &tx_id
                        )
                    })?;
                let current_pending_stage_num = tx.current_stage_index().ok_or(anyhow!(
                    "current stage not found for tx: {:?}, status error",
                    &tx_id
                ))? + 1; // index starts at 0 but we want to show 1 in email
                let tasks = process_email_template!(
                    self,
                    EmailTemplatePendingForApproval(TxRequiredApprovalEmailRequest {
                        from: from_wallet.clone().into(),
                        to: to.clone(),
                        amount: amount.try_to_f64()?,
                        asset: amount.asset_type(),
                        tx_link: tx_query_url,
                        step: current_pending_stage_num,
                    }),
                    &current_pending_stage_approvers,
                    &from_wallet,
                    false
                )?;
                email_tasks.extend(tasks);
            }
            PendingTx::ReadyForSigning(_tx) => {
                let tasks = process_email_template!(
                    self,
                    EmailTemplateReadyForSigning(TxBasicInfoEmailRequest {
                        from: from_wallet.clone().into(),
                        to: to.clone(),
                        amount: amount.try_to_f64()?,
                        asset: amount.asset_type(),
                        tx_link: tx_query_url,
                    }),
                    &operators,
                    &from_wallet,
                    false
                )?;
                email_tasks.extend(tasks);
            }
            PendingTx::DelegationExpired(_tx) => {
                let tasks = process_email_template!(
                    self,
                    EmailTemplateNeedReSign(TxBasicInfoEmailRequest {
                        from: from_wallet.clone().into(),
                        to: to.clone(),
                        amount: amount.try_to_f64()?,
                        asset: amount.asset_type(),
                        tx_link: tx_query_url,
                    }),
                    &operators,
                    &from_wallet,
                    false
                )?;
                email_tasks.extend(tasks);
            }
            _ => {
                return Err(anyhow!("not supported: {:?}", pending_tx));
            }
        };
        Ok(email_tasks)
    }

    // add nickname
    // convert email_response to email_task
    async fn finalize_email_task<T>(
        &self,
        recipients: HashSet<Email>,
        mut template: Box<dyn EmailTemplate<T = T>>,
        is_received_payment: bool,
        wallet_id: &WalletID,
    ) -> Result<Vec<SendEmailTask>>
    where
        T: EmailRequest,
    {
        let mut send_email_tasks = Vec::new();
        let (from_nickname, to_nickname): (Option<NicknameValue>, Option<NicknameValue>) =
            if is_received_payment {
                // for received payment:
                // "from" is an address on Address Book or unregistered address
                // "to" is a wallet ID, with a Wallet name as a nickname
                (
                    self.get_address_name(&template.from()).await?,
                    self.get_wallet_name(wallet_id).await?,
                )
            } else {
                // for history tx and pending tx:
                // "from" is a wallet ID, with a Wallet name as a nickname
                // "to" is an address on Address Book
                (
                    self.get_wallet_name(wallet_id).await?,
                    self.get_address_name(&template.to()).await?,
                )
            };

        // keep the NicknameValue definition for email content
        template.add_nickname(from_nickname, to_nickname);

        for recepient in recipients {
            send_email_tasks.push(template.to_email_task(recepient)?);
        }
        Ok(send_email_tasks)
    }

    async fn get_viewers_for_wallet(&self, wallet_id: &WalletID) -> Result<HashSet<Email>> {
        let wallet = self
            .db_client
            .get::<_, ConsoleWalletInfo>(wallet_id)
            .await?
            .ok_or_else(|| anyhow!("cannot find synced wallet info for: {:?}", wallet_id))?;
        Ok(wallet.viewers().deref().clone())
    }

    async fn get_associated_users(&self, wallet_id: &WalletID) -> Result<HashSet<Email>> {
        let wallet = self
            .db_client
            .get::<_, SyncedWalletInfo>(wallet_id)
            .await?
            .ok_or_else(|| anyhow!("cannot find synced wallet info for: {:?}", wallet_id))?;
        Ok(wallet.associated_users_owned())
    }

    async fn get_subscribed_recipients(
        &self,
        recepients: &HashSet<Email>,
        target_wallet: &WalletID,
    ) -> Result<HashSet<Email>> {
        let mut subscribed_recepients: HashSet<Email> = HashSet::new();
        for recepient in recepients {
            let subscription_info =
                match self.db_client.get::<_, SubscriptionInfo>(recepient).await? {
                    Some(subscription_info) => subscription_info,
                    None => {
                        log::debug!("no subscription info for recepient: {:?}", recepient);
                        continue;
                    }
                };
            if subscription_info.is_subscribed(target_wallet) {
                subscribed_recepients.insert(recepient.to_owned());
            } else {
                log::debug!(
                    "recepients: {:?} is not subscribed to target_wallet: {:?}",
                    recepient,
                    target_wallet
                );
            }
        }
        Ok(subscribed_recepients)
    }

    async fn get_address_name(&self, address: &str) -> Result<Option<NicknameValue>> {
        match self
            .db_client
            .get::<_, AddressBookEntry>(&address.to_string().to_lowercase())
            .await?
        {
            Some(address_book_entry) => {
                let name: String = address_book_entry.name.into();
                Ok(Some(name.into()))
            }
            None => Ok(None),
        }
    }

    async fn get_wallet_name(&self, wallet_id: &WalletID) -> Result<Option<NicknameValue>> {
        match self
            .db_client
            .get::<_, ConsoleWalletInfo>(wallet_id)
            .await?
        {
            Some(console_wallet_info) => {
                let name: String = console_wallet_info.wallet_name;
                Ok(Some(name.into()))
            }
            None => {
                log::error!("Can not find wallet name for wallet_id: {:?}", wallet_id);
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl Executable for EmailNotifyTask {
    async fn exec(&self) {
        if let Err(e) = self.inner_exec().await {
            log::error!("failed to execute email notify task: {:?}", e);
        }
    }
}
