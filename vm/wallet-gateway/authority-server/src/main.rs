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
use console_user::ConsoleUserRole;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::TryInto;
use std::path::Path;
use std::sync::Arc;
use url::Url;

use storable::console::{CreateConsoleWalletInfo, UpdateConsoleWalletInfo};
use storable::device_info::{BackupDeviceInfoBasic, DeviceInfoBasic};
use storable::user_info::UserInfo;
use types::external::Email;
use types::share::{DeviceID, Role, UserID, WalletID};
use utils::logger::setup_logger;

mod error;
use crate::error::{ErrorMessage, WebApiError, WebApiResult};
mod state_manager;
use crate::state_manager::SharedStateManager;
mod auth_data;
use crate::auth_data::AuthData;
mod console_user;
use crate::console_user::{Admin, ConsoleUser, ConsoleUserInfo};
mod console_wallet_info;
use crate::console_wallet_info::ConsoleWalletInfoForClient;
mod config;
mod file_utils;
use crate::config::AuthorityConfig;
mod tee_status_for_client;
use crate::tee_status_for_client::TeeStatusForClient;

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleUserRoleRequest {
    auth_data: AuthData, // the user logged in
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleUserRoleResponse {
    console_roles: HashSet<ConsoleUserRole>,
}

#[post("/console-user/role")]
async fn console_user_role(
    request: web::Json<ConsoleUserRoleRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<ConsoleUserRoleResponse>>> {
    let st = shared_read_ref(&req)?;
    let console_roles: HashSet<ConsoleUserRole> = match st.authenticate(&request.auth_data).await {
        Ok(email) => {
            log::debug!("console_user_role: login as {:?}", email);
            match TryInto::<ConsoleUser>::try_into(ConsoleUserInfo::new(email)) {
                Ok(console_user) => HashSet::from_iter(vec![console_user.get_role()]),
                Err(e) => {
                    log::warn!("user is not console user: {:?}", e);
                    HashSet::new()
                }
            }
        }
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    let response = ConsoleUserRoleResponse { console_roles };
    let result = WebApiResult::Ok(response);
    Ok(web::Json(result))
}

// user registry input
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UserBasicInfo {
    pub name: String,
    pub email: Email,
    pub roles: HashSet<Role>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserRegisterRequest {
    auth_data: AuthData,
    #[serde(flatten)]
    user_basic_info: UserBasicInfo,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserRegisterResponse {
    #[serde(flatten)]
    user_info: UserInfo,
}

#[post("/app-user/register")]
async fn user_register(
    request: web::Json<UserRegisterRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<UserRegisterResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("user_register: login as {:?}", admin);

    match st.register_user(&request.user_basic_info).await {
        Ok(info) => {
            let response = UserRegisterResponse { user_info: info };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("user_register error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserRoleAppendRequest {
    auth_data: AuthData, // the user logged in
    email: Email,        // the user to be modified
    roles: HashSet<Role>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserRoleAppendResponse {
    #[serde(flatten)]
    user_info: UserInfo,
}

#[post("/app-user/role/append")]
async fn user_role_append(
    request: web::Json<UserRoleAppendRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<UserRoleAppendResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("user_role_append: login as {:?}", admin);
    match st.append_role(&request.email, &request.roles).await {
        Ok(info) => {
            let response = UserRoleAppendResponse { user_info: info };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("user_role_append error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfoGetRequest {
    auth_data: AuthData,
    email: Email,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfoGetResponse {
    #[serde(flatten)]
    user_info: UserInfo,
}

#[post("/app-user/info/get")]
async fn user_info_get(
    request: web::Json<UserInfoGetRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<UserInfoGetResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("user_info_get: login as {:?}", admin);
    match st.get_user_info(&request.email).await {
        Ok(info) => {
            let response = UserInfoGetResponse { user_info: info };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("user_info_get error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfoGetAllRequest {
    auth_data: AuthData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfoGetAllResponse {
    user_list: Vec<UserInfo>,
}

#[post("/app-user/info/get-all")]
async fn user_info_get_all(
    request: web::Json<UserInfoGetAllRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<UserInfoGetAllResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("user_info_get_all: login as {:?}", admin);
    match st.get_all_user_info().await {
        Ok(user_list) => {
            let response = UserInfoGetAllResponse { user_list };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("user_info_get_all error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfoGetByIdRequest {
    auth_data: AuthData,
    id: UserID,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfoGetByIdResponse {
    #[serde(flatten)]
    user_info: UserInfo,
}

#[post("/app-user/info/get-by-id")]
async fn user_info_get_by_id(
    request: web::Json<UserInfoGetByIdRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<UserInfoGetByIdResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("user_info_get_by_id: login as {:?}", admin);
    match st.get_user_info_by_id(&request.id).await {
        Ok(info) => {
            let response = UserInfoGetByIdResponse { user_info: info };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("user_info_get_by_id error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceRegisterRequest {
    auth_data: AuthData,
    device_pubkeys: String, // base64
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceRegisterResponse {
    #[serde(flatten)]
    device_info: DeviceInfoBasic,
}
#[post("/device/register")]
async fn device_register(
    request: web::Json<DeviceRegisterRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<DeviceRegisterResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("device_register: login as {:?}", admin);
    // register the device on behalf of the logged-in user
    // cannot register device for other users
    match st
        .register_device(&admin.0.email, &request.device_pubkeys)
        .await
    {
        Ok(device_info) => {
            let response = DeviceRegisterResponse { device_info };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("device_register error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCertExportRequest {
    auth_data: AuthData,
    device_id: DeviceID,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCertExportResponse {
    download_link: Url,
}

#[post("/device/cert/export")]
async fn device_cert_export(
    request: web::Json<DeviceCertExportRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<DeviceCertExportResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("device_cert_export: login as {:?}", admin);
    // admin or device owner can export the device cert
    match st.export_device_cert(&request.device_id).await {
        Ok(download_link) => {
            let response = DeviceCertExportResponse { download_link };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("device_cert_export error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCertRefreshRequest {
    auth_data: AuthData,
    device_id: DeviceID,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCertRefreshResponse {
    download_link: Url,
}

#[post("/device/cert/refresh")]
async fn device_cert_refresh(
    request: web::Json<DeviceCertRefreshRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<DeviceCertRefreshResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("device_cert_refresh: login as {:?}", admin);

    // admin or device owner can export the device cert
    match st.refresh_device_cert(&request.device_id).await {
        Ok(download_link) => {
            let response = DeviceCertRefreshResponse { download_link };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("device_cert_refresh error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceAuthorizeRequest {
    auth_data: AuthData,
    from_device_id: DeviceID,
    backup_info: BackupDeviceInfoBasic,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceAuthorizeResponse {
    download_link: Url,
}

#[post("/device/authorize")]
async fn device_authorize(
    request: web::Json<DeviceAuthorizeRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<DeviceAuthorizeResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("device_authorize: login as {:?}", admin);
    // logged-in user should be the device owner or admin
    match st
        .authorize_backup_device(&request.from_device_id, &request.backup_info)
        .await
    {
        Ok(download_link) => {
            let response = DeviceAuthorizeResponse { download_link };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("device_authorize error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceInfoGetRequest {
    auth_data: AuthData,
    device_id: DeviceID,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceInfoGetResponse {
    #[serde(flatten)]
    device_info: DeviceInfoBasic,
}

#[post("/device/info/get")]
async fn device_info_get(
    request: web::Json<DeviceInfoGetRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<DeviceInfoGetResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("device_info_get: login as {:?}", admin);
    match st.get_device_info(&request.device_id).await {
        Ok(info) => {
            let response = DeviceInfoGetResponse { device_info: info };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("device_info_get error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceStatusRequest {
    auth_data: AuthData,
    device_id: DeviceID,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(transparent)]
struct DeviceStatusResponse {
    tee_status: TeeStatusForClient,
}

#[post("/device/status")]
async fn device_status(
    request: web::Json<DeviceStatusRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<DeviceStatusResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("device_status: login as {:?}", admin);
    match st.get_tee_status(&request.device_id).await {
        Ok(Some(tee_status)) => {
            let response = DeviceStatusResponse {
                tee_status: tee_status.into(),
            };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Ok(None) => {
            let response = DeviceStatusResponse {
                tee_status: TeeStatusForClient::Offline,
            };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("device_status error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceGetAllRequest {
    auth_data: AuthData,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceGetAllResponse {
    device_list: Vec<DeviceID>,
}

#[post("/device/get-all")]
async fn device_get_all(
    request: web::Json<DeviceGetAllRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<DeviceGetAllResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("device_current_online: login as {:?}", admin);
    match st.get_current_online_device().await {
        Ok(device_id) => {
            let response = DeviceGetAllResponse {
                device_list: vec![device_id],
            };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("device_current_online error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceSyncRequest {
    auth_data: AuthData,
    device_id: DeviceID,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceSyncResponse {
    message: String,
}

#[post("/device/sync")]
async fn device_sync(
    request: web::Json<DeviceSyncRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<DeviceSyncResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::info!(
        "device_sync: operated by {:?} for device {:?}",
        admin,
        request.device_id
    );
    match st.sync_config(&request.device_id).await {
        Ok(_) => {
            let response = DeviceSyncResponse {
                message: "Config updated, please wait for device to sync".to_string(),
            };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("device_sync error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletRegisterRequest {
    auth_data: AuthData,
    device_id: DeviceID,
    wallet_info_list: Vec<CreateConsoleWalletInfo>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletRegisterResponse {
    wallet_info_list: Vec<ConsoleWalletInfoForClient>,
}

#[post("/wallet/register")]
async fn wallet_register(
    request: web::Json<WalletRegisterRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletRegisterResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("wallet_register: login as {:?}", admin);
    // check all wallet_name length
    for wallet_info in &request.wallet_info_list {
        if wallet_info.wallet_name.len() > 20 {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(format!(
                "wallet_name length should be less than 20, got {}",
                wallet_info.wallet_name.len()
            )));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    }
    match st
        .register_wallets(&request.device_id, &request.wallet_info_list)
        .await
    {
        Ok(wallet_info_list) => {
            let mut console_wallet_info_list = Vec::new();
            for wallet_info in wallet_info_list {
                let account = match st.get_wallet_address(&wallet_info.wallet_id).await {
                    Ok(account) => account,
                    Err(e) => {
                        log::error!("wallet_register error: {:?}", e);
                        let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
                        return Ok(web::Json(WebApiResult::Error(error)));
                    }
                };
                console_wallet_info_list
                    .push(ConsoleWalletInfoForClient::new(wallet_info, account));
            }
            let response = WalletRegisterResponse {
                wallet_info_list: console_wallet_info_list,
            };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("wallet_register error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletInfoUpdateRequest {
    auth_data: AuthData,
    device_id: DeviceID,
    wallet_info_list: Vec<UpdateConsoleWalletInfo>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletInfoUpdateResponse {
    wallet_info_list: Vec<ConsoleWalletInfoForClient>,
}

#[post("/wallet/info/update")]
async fn wallet_info_update(
    request: web::Json<WalletInfoUpdateRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletInfoUpdateResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("wallet_info_update: login as {:?}", admin);
    // check all wallet_name length
    for wallet_info in &request.wallet_info_list {
        if wallet_info.wallet_name.len() > 20 {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(format!(
                "wallet_name length should be less than 20, got {}",
                wallet_info.wallet_name.len()
            )));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    }
    match st
        .update_wallet_info(&request.device_id, &request.wallet_info_list)
        .await
    {
        Ok(wallet_info_list) => {
            let mut console_wallet_info_list = Vec::new();
            for wallet_info in wallet_info_list {
                let account = match st.get_wallet_address(&wallet_info.wallet_id).await {
                    Ok(account) => account,
                    Err(e) => {
                        log::error!("wallet_register error: {:?}", e);
                        let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
                        return Ok(web::Json(WebApiResult::Error(error)));
                    }
                };
                console_wallet_info_list
                    .push(ConsoleWalletInfoForClient::new(wallet_info, account));
            }
            let response = WalletInfoUpdateResponse {
                wallet_info_list: console_wallet_info_list,
            };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("wallet_info_update error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletInfoGetRequest {
    auth_data: AuthData,
    wallet_id: WalletID,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletInfoGetResponse {
    wallet_info: ConsoleWalletInfoForClient,
}

#[post("/wallet/info/get")]
async fn wallet_info_get(
    request: web::Json<WalletInfoGetRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletInfoGetResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("wallet_info_get: login as {:?}", admin);
    match st.get_wallet_info(&request.wallet_id).await {
        Ok(wallet_info) => {
            let account = match st.get_wallet_address(&wallet_info.wallet_id).await {
                Ok(account) => account,
                Err(e) => {
                    log::error!("wallet_register error: {:?}", e);
                    let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
                    return Ok(web::Json(WebApiResult::Error(error)));
                }
            };
            let console_wallet_info = ConsoleWalletInfoForClient::new(wallet_info, account);
            let response = WalletInfoGetResponse {
                wallet_info: console_wallet_info,
            };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("wallet_info_get error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletInfoGetByDeviceRequest {
    auth_data: AuthData,
    device_id: DeviceID,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletInfoGetByDeviceResponse {
    wallet_info_list: Vec<ConsoleWalletInfoForClient>,
}

#[post("/wallet/info/get-by-device")]
async fn wallet_info_get_all(
    request: web::Json<WalletInfoGetByDeviceRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletInfoGetByDeviceResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("wallet_info_get_all: login as {:?}", admin);
    match st.get_all_wallet_info(&request.device_id).await {
        Ok(wallet_info_list) => {
            let mut console_wallet_info_list = Vec::new();
            for wallet_info in wallet_info_list {
                let account = match st.get_wallet_address(&wallet_info.wallet_id).await {
                    Ok(account) => account,
                    Err(e) => {
                        log::error!("wallet_register error: {:?}", e);
                        let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
                        return Ok(web::Json(WebApiResult::Error(error)));
                    }
                };
                console_wallet_info_list
                    .push(ConsoleWalletInfoForClient::new(wallet_info, account));
            }
            let response = WalletInfoGetByDeviceResponse {
                wallet_info_list: console_wallet_info_list,
            };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("wallet_info_get_all error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletCreateWithIdRequest {
    auth_data: AuthData,
    device_id: DeviceID,
    wallet_info_list: Vec<UpdateConsoleWalletInfo>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletCreateWithIdResponse {
    wallet_info_list: Vec<ConsoleWalletInfoForClient>,
}

#[post("/wallet/create/with-id")]
async fn wallet_create_with_id(
    request: web::Json<WalletCreateWithIdRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletCreateWithIdResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("wallet_create_with_id: login as {:?}", admin);
    // check all wallet_name length
    for wallet_info in &request.wallet_info_list {
        if wallet_info.wallet_name.len() > 20 {
            let error = WebApiError::InvalidOperation(ErrorMessage::new(format!(
                "wallet_name length should be less than 20, got {}",
                wallet_info.wallet_name.len()
            )));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    }
    match st
        .wallet_create_with_id(&request.device_id, &request.wallet_info_list)
        .await
    {
        Ok(wallet_info_list) => {
            let mut console_wallet_info_list = Vec::new();
            for wallet_info in wallet_info_list {
                let account = match st.get_wallet_address(&wallet_info.wallet_id).await {
                    Ok(account) => account,
                    Err(e) => {
                        log::error!("wallet_register error: {:?}", e);
                        let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
                        return Ok(web::Json(WebApiResult::Error(error)));
                    }
                };
                console_wallet_info_list
                    .push(ConsoleWalletInfoForClient::new(wallet_info, account));
            }
            let response = WalletCreateWithIdResponse {
                wallet_info_list: console_wallet_info_list,
            };
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("wallet_info_update error: {:?}", e);
            let error = WebApiError::InvalidOperation(ErrorMessage::new(e.to_string()));
            Ok(web::Json(WebApiResult::Error(error)))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletInfoRevertRequest {
    auth_data: AuthData,
    wallet_id: WalletID,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletInfoRevertResponse {}

#[post("/wallet/info/revert")]
async fn wallet_info_revert(
    request: web::Json<WalletInfoRevertRequest>,
    req: HttpRequest,
) -> Result<web::Json<WebApiResult<WalletInfoRevertResponse>>> {
    let st = shared_read_ref(&req)?;
    let admin: Admin = match st.authenticate_console_user(&request.auth_data).await {
        Ok(admin) => admin,
        Err(e) => {
            let error = WebApiError::Unauthorized(ErrorMessage::new(e.to_string()));
            return Ok(web::Json(WebApiResult::Error(error)));
        }
    };
    log::debug!("wallet_info_revert: login as {:?}", admin);

    match st.revert_wallet_info(&request.wallet_id).await {
        Ok(_) => {
            let response = WalletInfoRevertResponse {};
            let result = WebApiResult::Ok(response);
            Ok(web::Json(result))
        }
        Err(e) => {
            log::error!("wallet_info_revert error: {:?}", e);
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
    let authority_config = AuthorityConfig::new(
        Path::new(&root_path),
        Path::new("config/runtime_config.toml"),
    );

    setup_logger(&authority_config.log_path, "authority-server")
        .expect("Failed to initialize logger");

    let ip = authority_config.ip;
    let port = authority_config.port;
    // shared state manager
    let st = Arc::new(
        SharedStateManager::new(Arc::new(authority_config))
            .await
            .map_err(|e| {
                log::error!("SharedStateManager::new() error: {:?}", e);
                std::io::Error::new(std::io::ErrorKind::Other, e)
            })?,
    );
    let app_state = web::Data::new(st.clone());

    // start server
    log::info!("starting HTTP server at http://{}:{}", ip, port);
    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .app_data(web::JsonConfig::default().limit(4096)) // <- limit size of the payload (global configuration)
            .app_data(web::Data::clone(&app_state))
            .service(web::resource("/index.html").to(index))
            .service(console_user_role)
            .service(user_register)
            .service(user_role_append)
            .service(user_info_get)
            .service(user_info_get_by_id)
            .service(user_info_get_all)
            .service(device_register)
            .service(device_cert_export)
            .service(device_cert_refresh)
            .service(device_authorize)
            .service(device_info_get)
            .service(device_get_all)
            .service(device_status)
            .service(device_sync)
            .service(wallet_register)
            .service(wallet_create_with_id)
            .service(wallet_info_get)
            .service(wallet_info_get_all)
            .service(wallet_info_update)
            .service(wallet_info_revert)
    })
    .bind((ip, port))?
    .run()
    .await
}
