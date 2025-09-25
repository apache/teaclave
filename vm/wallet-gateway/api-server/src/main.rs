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

use actix_web::{
    http::header::ContentType, middleware, post, web, App, HttpRequest, HttpResponse, HttpServer,
    Result,
};
use anyhow::anyhow;
use client_info::{
    AdditionalInfoForClient, ApprovalStageInfo, ClientTxTransfer, FeeEstimationRequest,
    FeeEstimationResponse, HistoryTxInfo, PendingTxInfo, TxHistoryReceivedPayment,
    WalletInfoForClient,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use storable::address_book::{AddressBookEntry, AddressName};
use storable::delegation::DelegationInfo;
use storable::nickname::{NicknameKey, NicknameValue};
use storable::pending_tx::PendingTx;
use storable::user_info::User;
use types::external::ClientExternalAddress;
use types::share::{TransactionID, WalletID};
use utils::logger::setup_logger;

use webapi::*;

/// simple handle
async fn index(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().content_type(ContentType::html()).body(
        "<!DOCTYPE html><html><body>\
            <p>Welcome to your TLS-secured homepage!</p>\
        </body></html>",
    )
}

fn shared_read_ref(req: &HttpRequest) -> Result<&SharedStateManager> {
    let shared_state = req
        .app_data::<web::Data<Arc<SharedStateManager>>>()
        .ok_or_else(|| {
            log::error!("shared_read_ref error: {:?}", anyhow!("no shared state"));
            actix_web::error::ErrorInternalServerError("no shared state")
        })?;
    Ok(shared_state.get_ref())
}

macro_rules! authenticate {
    ($st :expr, $auth_data :expr) => {
        match $st.authenticate($auth_data).await {
            Ok(operator) => operator,
            Err(e) => {
                let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
                return Ok(web::Json(WebApiResult::Error(error)));
            }
        }
    };
}

// to avoid code duplication
macro_rules! get_additional_info {
    ($st :expr, $transfer_info :expr) => {
        match $st.get_additional_info($transfer_info).await {
            Ok((from_info, to_info)) => (from_info, to_info),
            Err(e) => {
                let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
                return Ok(web::Json(WebApiResult::Error(error)));
            }
        }
    };
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxCreateRequest {
    auth_data: AuthData,
    tx_transfer: ClientTxTransfer,
    address_name: AddressBookEntry,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxCreateResponse {
    tx_id: TransactionID,
}

impl TxCreateResponse {
    fn new(tx_id: TransactionID) -> Self {
        Self { tx_id }
    }
}

#[post("/tx/create")]
async fn tx_create(
    request: web::Json<TxCreateRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<TxCreateResponse>>> {
    let st = shared_read_ref(&req)?;
    let operator = authenticate!(st, &request.auth_data);

    let tx_id = match st
        .create_tx(operator, request.tx_transfer.clone(), &request.address_name)
        .await
    {
        Ok(tx_id) => tx_id,
        Err(e) => {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    let response = TxCreateResponse::new(tx_id);
    Ok(web::Json(WebApiResult::Ok(response)))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxApproveRequest {
    auth_data: AuthData,
    tx_id: TransactionID,
    approval_chain: Vec<ApprovalStageInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxApproveResponse {
    tx_id: TransactionID,
}

impl TxApproveResponse {
    fn new(tx_id: TransactionID) -> Self {
        Self { tx_id }
    }
}

#[post("/tx/approve")]
async fn tx_approve(
    request: web::Json<TxApproveRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<TxApproveResponse>>> {
    let request = request.into_inner();
    let st = shared_read_ref(&req)?;
    let approver = authenticate!(st, &request.auth_data);

    let tx_id = match st
        .approve_tx(&approver, &request.tx_id, request.approval_chain)
        .await
    {
        Ok(tx) => tx,
        Err(e) => {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::info!(
        "tx_approve: approver {:?} tx_id {:?}",
        approver,
        request.tx_id
    );

    let pending_tx_response = TxApproveResponse::new(tx_id);

    Ok(web::Json(WebApiResult::Ok(pending_tx_response)))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxRejectRequest {
    auth_data: AuthData,
    tx_id: TransactionID,
    approval_chain: Vec<ApprovalStageInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxRejectResponse {
    tx_id: TransactionID,
}

impl TxRejectResponse {
    fn new(tx_id: TransactionID) -> Self {
        Self { tx_id }
    }
}

#[post("/tx/reject")]
async fn tx_reject(
    request: web::Json<TxRejectRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<TxRejectResponse>>> {
    let request = request.into_inner();
    let st = shared_read_ref(&req)?;
    let approver = authenticate!(st, &request.auth_data);

    let tx_id = match st
        .reject_tx(&approver, &request.tx_id, request.approval_chain)
        .await
    {
        Ok(tx) => tx,
        Err(e) => {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };

    log::info!(
        "tx_reject: approver {:?} tx_id {:?}",
        approver,
        request.tx_id
    );

    Ok(web::Json(WebApiResult::Ok(TxRejectResponse::new(tx_id))))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxRecallRequest {
    auth_data: AuthData,
    tx_id: TransactionID,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxRecallResponse {
    tx_id: TransactionID,
}

impl TxRecallResponse {
    fn new(tx_id: TransactionID) -> Self {
        Self { tx_id }
    }
}

#[post("/tx/recall")]
async fn tx_recall(
    request: web::Json<TxRecallRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<TxRecallResponse>>> {
    let st = shared_read_ref(&req)?;
    let operator = authenticate!(st, &request.auth_data);

    log::info!(
        "tx_recall: operator {:?} tx_id {:?}",
        operator,
        request.tx_id
    );
    let tx_id = match st.recall_tx(&operator, &request.tx_id).await {
        Ok(tx_id) => tx_id,
        Err(e) => {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };

    Ok(web::Json(WebApiResult::Ok(TxRecallResponse::new(tx_id))))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "pendingTxStatus")]
enum PendingTxResponse {
    PendingForApproval(PendingTxInfo),
    ReadyForSigning(PendingTxInfo),
    DelegationStarted(PendingTxInfo),
    DelegationExpired(PendingTxInfo),
    RejectedByApprover(PendingTxInfo),
}
impl PendingTxResponse {
    pub fn new(
        pending_tx: PendingTx,
        from_info: AdditionalInfoForClient,
        to_info: AdditionalInfoForClient,
    ) -> Self {
        match pending_tx {
            PendingTx::PendingForApproval(tx) => {
                PendingTxResponse::PendingForApproval(PendingTxInfo::new(tx, from_info, to_info))
            }
            PendingTx::ReadyForSigning(tx) => {
                PendingTxResponse::ReadyForSigning(PendingTxInfo::new(tx, from_info, to_info))
            }
            PendingTx::DelegationStarted(tx) => {
                PendingTxResponse::DelegationStarted(PendingTxInfo::new(tx, from_info, to_info))
            }
            PendingTx::DelegationExpired(tx) => {
                PendingTxResponse::DelegationExpired(PendingTxInfo::new(tx, from_info, to_info))
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxListPendingRequest {
    auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxListPendingResponse {
    txs: Vec<PendingTxResponse>,
}

impl TxListPendingResponse {
    fn new(txs: Vec<PendingTxResponse>) -> Self {
        Self { txs }
    }
}

#[post("/tx/list/pending")]
async fn tx_list_pending(
    request: web::Json<TxListPendingRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<TxListPendingResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    let mut txs = match st.list_pending_tx_for_user(&user).await {
        Ok(txs) => txs,
        Err(e) => {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    txs.sort_by_key(|p| std::cmp::Reverse(p.tx().get_created_at()));

    let mut items = Vec::new();
    for pending_tx in txs {
        let (from_info, to_info) = get_additional_info!(st, pending_tx.tx().transfer_info());
        items.push(PendingTxResponse::new(pending_tx, from_info, to_info));
    }

    let response = TxListPendingResponse::new(items);
    Ok(web::Json(WebApiResult::Ok(response)))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxListIdRequest {
    auth_data: AuthData,
    tx_id: TransactionID,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status")]
enum TxListIdResponse {
    Pending(PendingTxResponse),
    History(HistoryTxInfo),
}

#[post("/tx/list/id")]
async fn tx_list_id(
    request: web::Json<TxListIdRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<TxListIdResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    match st
        .get_pending_tx_for_user(user.get_email(), &request.tx_id)
        .await
    {
        Ok(tx) => {
            let (from_info, to_info) = get_additional_info!(st, tx.tx().transfer_info());
            let pending_tx_response = PendingTxResponse::new(tx, from_info, to_info);

            Ok(web::Json(WebApiResult::Ok(TxListIdResponse::Pending(
                pending_tx_response,
            ))))
        }
        Err(e) => {
            log::info!(
                "get_pending_tx_for_user error: {:?}, trying to load from history db",
                e
            );
            match st.get_signed_tx_for_user(&user, &request.tx_id).await {
                Ok(tx) => {
                    let explorer_base_url = tx
                        .tx
                        .transfer_info
                        .amount
                        .asset_type()
                        .as_ck_network(st.network_config().network_type)
                        .explorer_base_url();
                    let (from_info, to_info) = get_additional_info!(st, tx.tx.transfer_info());
                    let response = TxListIdResponse::History(
                        HistoryTxInfo::new(tx, explorer_base_url, from_info, to_info).map_err(
                            |e| {
                                log::error!("Failed to create HistoryTxInfo: {}", e);
                                actix_web::error::ErrorInternalServerError(e)
                            },
                        )?,
                    );
                    Ok(web::Json(WebApiResult::Ok(response)))
                }
                Err(e) => {
                    let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
                    Ok(web::Json(WebApiResult::Error(error)))
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxListHistoryRequest {
    auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxListHistoryResponse {
    txs: Vec<HistoryTxInfo>,
}

impl TxListHistoryResponse {
    fn new(txs: Vec<HistoryTxInfo>) -> Self {
        Self { txs }
    }
}

#[post("/tx/list/history")]
async fn tx_list_history(
    request: web::Json<TxListHistoryRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<TxListHistoryResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    let mut history_vec = match st.list_signed_tx_for_user(&user).await {
        Ok(history_vec) => history_vec,
        Err(e) => {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };

    // sort the HistoryTx by created_at
    history_vec.sort_by_key(|p| std::cmp::Reverse(p.tx().get_created_at()));

    let mut items = Vec::new();
    for tx_history in history_vec {
        let explorer_base_url = tx_history
            .tx
            .transfer_info()
            .amount
            .asset_type()
            .as_ck_network(st.network_config().network_type)
            .explorer_base_url();
        let (from_info, to_info) = get_additional_info!(st, tx_history.tx.transfer_info());
        items.push(
            HistoryTxInfo::new(tx_history, explorer_base_url, from_info, to_info).map_err(|e| {
                log::error!("Failed to create HistoryTxInfo: {}", e);
                actix_web::error::ErrorInternalServerError(e)
            })?,
        );
    }
    let response = TxListHistoryResponse::new(items);
    Ok(web::Json(WebApiResult::Ok(response)))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxHistoryReceivedRequest {
    auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxHistoryReceivedResponse {
    txs: Vec<TxHistoryReceivedPayment>,
}
impl TxHistoryReceivedResponse {
    fn new(txs: Vec<TxHistoryReceivedPayment>) -> Self {
        Self { txs }
    }
}

#[post("/tx/list/history/received")]
async fn tx_list_history_received(
    request: web::Json<TxHistoryReceivedRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<TxHistoryReceivedResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    let received_payments = match st.gather_user_history_tx(&user).await {
        Ok(received_payments) => received_payments,
        Err(e) => {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };

    Ok(web::Json(WebApiResult::Ok(TxHistoryReceivedResponse::new(
        received_payments,
    ))))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletListRequest {
    auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletListResponse {
    wallets: Vec<WalletInfoForClient>,
}

impl WalletListResponse {
    fn new(wallets: Vec<WalletInfoForClient>) -> Self {
        Self { wallets }
    }
}

#[post("/wallet/list")]
async fn wallet_list(
    request: web::Json<WalletListRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletListResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    match st.gather_user_wallets_info(&user).await {
        Ok(wallets_info) => {
            let response = WalletListResponse::new(wallets_info);
            Ok(web::Json(WebApiResult::Ok(response)))
        }
        Err(e) => {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserSetNicknameRequest {
    auth_data: AuthData,
    key: NicknameKey,
    nickname: Option<NicknameValue>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserSetNicknameResponse {}

#[post("/user/nickname/set")]
async fn user_set_nickname(
    request: web::Json<UserSetNicknameRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<UserSetNicknameResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    let request = request.into_inner();
    match request.nickname {
        Some(nickname) => {
            st.set_nickname(&user, request.key, nickname)
                .await
                .map_err(|e| {
                    log::error!("set_nickname error: {:?}", e);
                    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                })?;
        }
        None => {
            st.remove_nickname(&user, &request.key).await.map_err(|e| {
                log::error!("remove_nickname error: {:?}", e);
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;
        }
    }

    Ok(web::Json(WebApiResult::Ok(UserSetNicknameResponse {})))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserListNicknameRequest {
    auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserListNicknameResponse {
    nicknames: HashMap<NicknameKey, NicknameValue>,
}

#[post("/user/nickname/list")]
async fn user_list_nickname(
    request: web::Json<UserListNicknameRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<UserListNicknameResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    match st.get_all_nickname_for_user(&user).await {
        Ok(nicknames) => {
            let response = UserListNicknameResponse { nicknames };
            Ok(web::Json(WebApiResult::Ok(response)))
        }
        Err(e) => {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletSubscribeRequest {
    auth_data: AuthData,
    wallet_id: WalletID,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletSubscribeResponse {}

#[post("/wallet/subscribe")]
async fn wallet_subscribe(
    request: web::Json<WalletSubscribeRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletSubscribeResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    match st.subscribe(&user, &request.wallet_id).await {
        Ok(_) => {
            log::info!(
                "subscribed account {:?} for user {:?}",
                &request.wallet_id,
                &user.get_email()
            );
            Ok(web::Json(WebApiResult::Ok(WalletSubscribeResponse {})))
        }
        Err(e) => {
            log::error!("subscribe error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletUnsubscribeRequest {
    auth_data: AuthData,
    wallet_id: WalletID,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletUnsubscribeResponse {}

#[post("/wallet/unsubscribe")]
async fn wallet_unsubscribe(
    request: web::Json<WalletUnsubscribeRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletUnsubscribeResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    match st.unsubscribe(&user, &request.wallet_id).await {
        Ok(_) => {
            log::info!(
                "unsubscribed account {:?} for user {:?}",
                &request.wallet_id,
                &user.get_email()
            );
            Ok(web::Json(WebApiResult::Ok(WalletUnsubscribeResponse {})))
        }
        Err(e) => {
            log::error!("unsubscribe error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletSubscribeAllRequest {
    auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletSubscribeAllResponse {}

#[post("/wallet/subscribe/all")]
async fn wallet_subscribe_all(
    request: web::Json<WalletSubscribeAllRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletSubscribeAllResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    match st.subscribe_all_associated_wallets(&user).await {
        Ok(_) => {
            log::info!(
                "subscribed all accounts for user {:?}",
                &user.get_info().get_email()
            );
            Ok(web::Json(WebApiResult::Ok(WalletSubscribeAllResponse {})))
        }
        Err(e) => {
            log::error!("subscribe_all error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletUnsubscribeAllRequest {
    auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletUnsubscribeAllResponse {}

#[post("/wallet/unsubscribe/all")]
async fn wallet_unsubscribe_all(
    request: web::Json<WalletUnsubscribeAllRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletUnsubscribeAllResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = authenticate!(st, &request.auth_data);

    match st.unsubscribe_all(&user).await {
        Ok(_) => {
            log::info!(
                "unsubscribed all accounts for user {:?}",
                &user.get_info().get_email()
            );
            Ok(web::Json(WebApiResult::Ok(WalletUnsubscribeAllResponse {})))
        }
        Err(e) => {
            log::error!("unsubscribe_all error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxSignRequest {
    auth_data: AuthData,
    tx_id: TransactionID,
    delegation_info: Option<DelegationInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxFeeEstimateRequest {
    auth_data: AuthData,
    tx_info_for_gas: FeeEstimationRequest,
}

#[post("/tx/fee/estimate")]
async fn tx_fee_estimate(
    request: web::Json<TxFeeEstimateRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<FeeEstimationResponse>>> {
    let st = shared_read_ref(&req)?;
    let _user: User = authenticate!(st, &request.auth_data);
    let request = request.into_inner();

    match st.estimate_tx_fee(request.tx_info_for_gas).await {
        Ok(fee) => Ok(web::Json(WebApiResult::Ok(fee))),
        Err(e) => {
            log::error!("estimate_gas error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressBookAddRequest {
    auth_data: AuthData,
    address: ClientExternalAddress,
    name: AddressName,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressBookAddResponse {
    entry: AddressBookEntry,
}

#[post("/address-book/add")]
async fn address_book_add(
    request: web::Json<AddressBookAddRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<AddressBookAddResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = match st.authenticate(&request.auth_data).await {
        Ok(user) => user,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };

    match st
        .add_address_book_entry(&user, &request.address, &request.name)
        .await
    {
        Ok(entry) => Ok(web::Json(WebApiResult::Ok(AddressBookAddResponse {
            entry,
        }))),
        Err(e) => {
            log::error!("add_address_book_entry error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressBookUpdateRequest {
    auth_data: AuthData,
    existing_entry: AddressBookEntry,
    new_name: Option<AddressName>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressBookUpdateResponse {}

#[post("/address-book/update")]
async fn address_book_update(
    request: web::Json<AddressBookUpdateRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<AddressBookUpdateResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = match st.authenticate(&request.auth_data).await {
        Ok(user) => user,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    match request.new_name {
        Some(ref name) => {
            // update name
            match st
                .update_address_book_entry(&user, &request.existing_entry, name)
                .await
            {
                Ok(_) => Ok(web::Json(WebApiResult::Ok(AddressBookUpdateResponse {}))),
                Err(e) => {
                    log::error!("update_address_book_entry error: {:?}", e);
                    let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
                    Ok(web::Json(WebApiResult::Error(error)))
                }
            }
        }
        None => {
            // clear name
            match st
                .remove_address_book_entry(&user, &request.existing_entry)
                .await
            {
                Ok(_) => Ok(web::Json(WebApiResult::Ok(AddressBookUpdateResponse {}))),
                Err(e) => {
                    log::error!("remove_address_book_entry error: {:?}", e);
                    let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
                    Ok(web::Json(WebApiResult::Error(error)))
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressBookListRequest {
    auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressBookListResponse {
    entries: Vec<AddressBookEntry>,
}

#[post("/address-book/list")]
async fn address_book_list(
    request: web::Json<AddressBookListRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<AddressBookListResponse>>> {
    let st = shared_read_ref(&req)?;
    let user: User = match st.authenticate(&request.auth_data).await {
        Ok(user) => user,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };

    match st.list_address_book_entries(&user).await {
        Ok(entries) => Ok(web::Json(WebApiResult::Ok(AddressBookListResponse {
            entries,
        }))),
        Err(e) => {
            log::error!("list_address_book_entries error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_cors::Cors;

    // setup global config
    let root_path = std::env::var("CK_ROOT_PATH").map_err(|e| {
        log::error!("CK_ROOT_PATH error: {:?}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;
    let webapi_config = WebapiConfig::new(
        Path::new(&root_path),
        Path::new("config/runtime_config.toml"),
    )
    .map_err(|e| {
        log::error!("WebapiConfig::new() error: {:?}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    // setup logger
    setup_logger(&webapi_config.shared_state_config().log_path, "api-server")
        .expect("Failed to initialize logger");

    // shared state manager
    let st = Arc::new(
        SharedStateManager::new(webapi_config.shared_state_config())
            .await
            .map_err(|e| {
                log::error!("SharedStateManager::new() error: {:?}", e);
                std::io::Error::new(std::io::ErrorKind::Other, e)
            })?,
    );

    let app_state = web::Data::new(st.clone());

    // start server
    let http_server_config = webapi_config.http_server_config();
    let ip = http_server_config.ip();
    let port = http_server_config.port();
    log::info!("starting HTTPS server at https://{}:{}", ip, port);

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .app_data(web::JsonConfig::default().limit(4096)) // <- limit size of the payload (global configuration)
            .app_data(web::Data::clone(&app_state))
            .service(web::resource("/index.html").to(index))
            .service(wallet_list)
            .service(wallet_subscribe)
            .service(wallet_unsubscribe)
            .service(wallet_subscribe_all)
            .service(wallet_unsubscribe_all)
            .service(user_set_nickname)
            .service(user_list_nickname)
            .service(tx_fee_estimate)
            .service(tx_create)
            .service(tx_approve)
            .service(tx_reject)
            .service(tx_recall)
            .service(tx_list_pending)
            .service(tx_list_id)
            .service(tx_list_history)
            .service(tx_list_history_received)
            .service(address_book_add)
            .service(address_book_list)
            .service(address_book_update)
    })
    .bind((ip, port))?
    .run()
    .await
}
