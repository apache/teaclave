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

use crate::email_task::SendEmailTask;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use storable::nickname::NicknameValue;
use types::external::{AssetType, Email};
use types::serde_util;
use url::Url;

fn format_nickname_address(nickname: Option<NicknameValue>, address: &str) -> String {
    match nickname {
        Some(nickname) => format!("{} ({})", nickname.as_str(), address),
        None => address.to_string(),
    }
}

pub trait EmailTemplate: Send + Sync {
    type T: EmailRequest;

    fn template_name(&self) -> String;
    fn inner(&self) -> &Self::T;
    fn inner_mut(&mut self) -> &mut Self::T;
    fn from(&self) -> String {
        self.inner().from().to_string()
    }
    fn to(&self) -> String {
        self.inner().to().to_string()
    }
    fn template_content_to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self.inner())
    }
    fn add_nickname(
        &mut self,
        from_nickname: Option<NicknameValue>,
        to_nickname: Option<NicknameValue>,
    ) {
        self.inner_mut().add_nickname(from_nickname, to_nickname);
    }
    fn to_email_task(&self, email: Email) -> Result<SendEmailTask> {
        Ok(SendEmailTask {
            email,
            template_name: self.template_name(),
            email_template: self.template_content_to_json()?,
        })
    }
}

macro_rules! impl_email_template {
    // template struct, template parameter (for mailgun), template name (for mailgun)
    ($struct_name:ident, $inner_type:ty, $template_name:expr) => {
        impl EmailTemplate for $struct_name {
            type T = $inner_type;

            fn template_name(&self) -> String {
                $template_name.to_string()
            }

            fn inner(&self) -> &Self::T {
                &self.0
            }

            fn inner_mut(&mut self) -> &mut Self::T {
                &mut self.0
            }
        }
    };
}

// mail templates which uses basic info
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailTemplateCancelled(pub TxBasicInfoEmailRequest);
impl_email_template!(
    EmailTemplateCancelled,
    TxBasicInfoEmailRequest,
    "tx-cancelled"
);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailTemplateDeclined(pub TxBasicInfoEmailRequest);
impl_email_template!(
    EmailTemplateDeclined,
    TxBasicInfoEmailRequest,
    "tx-declined"
);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailTemplateReadyForSigning(pub TxBasicInfoEmailRequest);
impl_email_template!(
    EmailTemplateReadyForSigning,
    TxBasicInfoEmailRequest,
    "ready-to-sign"
);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailTemplateNeedReSign(pub TxBasicInfoEmailRequest);
impl_email_template!(EmailTemplateNeedReSign, TxBasicInfoEmailRequest, "re-sign");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailTemplateTxCreated(pub TxBasicInfoEmailRequest);
impl_email_template!(
    EmailTemplateTxCreated,
    TxBasicInfoEmailRequest,
    "tx-created"
);

// templates which uses specific info
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailTemplatePendingForApproval(pub TxRequiredApprovalEmailRequest);
impl_email_template!(
    EmailTemplatePendingForApproval,
    TxRequiredApprovalEmailRequest,
    "require-approval"
);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailTemplateOnChain(pub TxCompletedEmailRequest);
impl_email_template!(
    EmailTemplateOnChain,
    TxCompletedEmailRequest,
    "tx-completed"
);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailTemplateReceived(pub TxReceivedEmailRequest);
impl_email_template!(EmailTemplateReceived, TxReceivedEmailRequest, "tx-received");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailTemplateRejectedByNetwork(pub TxNetworkEmailRequest);
impl_email_template!(
    EmailTemplateRejectedByNetwork,
    TxNetworkEmailRequest,
    "tx-rejected-by-network"
);

pub trait EmailRequest: Serialize {
    fn from(&self) -> &str;
    fn to(&self) -> &str;
    fn set_from(&mut self, from: String);
    fn set_to(&mut self, to: String);
    fn add_nickname(
        &mut self,
        from_nickname: Option<NicknameValue>,
        to_nickname: Option<NicknameValue>,
    ) {
        self.set_from(format_nickname_address(from_nickname, self.from()));
        self.set_to(format_nickname_address(to_nickname, self.to()));
    }
}

macro_rules! impl_email_request_get_set {
    ($struct_name:ident) => {
        impl EmailRequest for $struct_name {
            fn from(&self) -> &str {
                &self.from
            }

            fn to(&self) -> &str {
                &self.to
            }

            fn set_from(&mut self, from: String) {
                self.from = from;
            }

            fn set_to(&mut self, to: String) {
                self.to = to;
            }
        }
    };
}

// PendingForApproval
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TxRequiredApprovalEmailRequest {
    pub from: String, // AccountId(Nickname)
    pub to: String,
    #[serde(with = "serde_util::f64_string")]
    pub amount: f64,
    pub asset: AssetType,
    #[serde(with = "serde_util::url_string_for_email")]
    pub tx_link: Url,
    pub step: usize,
}
impl_email_request_get_set!(TxRequiredApprovalEmailRequest);

// HistoryEvent(Failed) + ReadyForSigning
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TxBasicInfoEmailRequest {
    pub from: String,
    pub to: String,
    #[serde(with = "serde_util::f64_string")]
    pub amount: f64,
    pub asset: AssetType,
    #[serde(with = "serde_util::url_string_for_email")]
    pub tx_link: Url,
}
impl_email_request_get_set!(TxBasicInfoEmailRequest);

// Onchain
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TxCompletedEmailRequest {
    pub from: String,
    pub to: String,
    #[serde(with = "serde_util::f64_string")]
    pub amount: f64,
    pub asset: AssetType,
    #[serde(with = "serde_util::url_string_for_email")]
    pub etherscan_link: Url,
    #[serde(with = "serde_util::url_string_for_email")]
    pub tx_link: Url,
}
impl_email_request_get_set!(TxCompletedEmailRequest);

// Received
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TxReceivedEmailRequest {
    pub from: String,
    pub to: String,
    #[serde(with = "serde_util::f64_string")]
    pub amount: f64,
    pub asset: AssetType,
    #[serde(with = "serde_util::url_string_for_email")]
    pub etherscan_link: Url,
}
impl_email_request_get_set!(TxReceivedEmailRequest);

// Rejected by Network
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TxNetworkEmailRequest {
    pub from: String,
    pub to: String,
    #[serde(with = "serde_util::f64_string")]
    pub amount: f64,
    pub asset: AssetType,
    pub message: String,
    #[serde(with = "serde_util::url_string_for_email")]
    pub tx_link: Url,
}
impl_email_request_get_set!(TxNetworkEmailRequest);
