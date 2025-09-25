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

use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};
use anyhow::{anyhow, Result};
use rocksdb::{DBWithThreadMode, IteratorMode, MultiThreaded};
use std::{collections::HashMap, sync::Arc};

use ck_config::RuntimeConfig;
use db_manager::local_service_client::{
    DeleteRequest, DeleteResponse, GetRequest, GetResponse, ListByPrefixRequest,
    ListByPrefixResponse, ListRequest, ListResponse, PutRequest, PutResponse,
};

#[derive(Debug, Clone)]
struct LocalStorageService {
    db: Arc<DBWithThreadMode<MultiThreaded>>,
    listening_address: String,
}

impl LocalStorageService {
    pub fn init(db_path: std::path::PathBuf, ip: std::net::IpAddr, port: u16) -> Result<Self> {
        let db = DBWithThreadMode::<MultiThreaded>::open_default(db_path)?;
        let addr = format!("{}:{}", ip, port);
        Ok(Self {
            db: Arc::new(db),
            listening_address: addr,
        })
    }

    pub async fn start(&self) -> std::io::Result<()> {
        let app_data = web::Data::new(self.clone());
        HttpServer::new(move || {
            App::new()
                .wrap(middleware::Logger::default())
                .app_data(app_data.clone())
                .route("/get", web::post().to(handle_get))
                .route("/put", web::post().to(handle_put))
                .route("/delete", web::post().to(handle_delete))
                .route("/list", web::post().to(handle_list))
                .route("/list_by_prefix", web::post().to(handle_list_by_prefix))
        })
        .bind(&self.listening_address)?
        .run()
        .await
    }

    fn get(&self, request: GetRequest) -> Result<GetResponse> {
        self.db
            .get(request.key)
            .map_err(|e| anyhow!("rocksdb get error: {}", e))
            .map(|value| GetResponse { value })
    }

    fn put(&self, request: PutRequest) -> Result<PutResponse> {
        self.db
            .put(&request.key, &request.value)
            .map_err(|e| anyhow!("rocksdb put error: {}", e))
            .map(|_| PutResponse {})
    }

    fn delete(&self, request: DeleteRequest) -> Result<DeleteResponse> {
        self.db
            .delete(request.key)
            .map_err(|e| anyhow!("rocksdb delete error: {}", e))
            .map(|_| DeleteResponse {})
    }

    fn list(&self) -> Result<ListResponse> {
        let mut map: HashMap<String, Vec<u8>> = HashMap::new();
        let iter = self.db.iterator(IteratorMode::Start);
        for item in iter {
            let (key, value) = item?;
            map.insert(String::from_utf8(key.to_vec())?, value.to_vec());
        }
        Ok(ListResponse { map })
    }

    fn list_by_prefix(&self, request: ListByPrefixRequest) -> Result<ListByPrefixResponse> {
        let prefix = &request.prefix;

        let mut map: HashMap<String, Vec<u8>> = HashMap::new();
        let iter = self.db.iterator(IteratorMode::Start);
        for item in iter {
            let (key, value) = item?;

            let key = String::from_utf8(key.to_vec())?;
            if key.starts_with(prefix) {
                map.insert(key, value.to_vec());
            }
        }
        Ok(ListByPrefixResponse { map })
    }
}

async fn handle_get(
    data: web::Data<LocalStorageService>,
    request: web::Json<GetRequest>,
) -> impl Responder {
    match data.get(request.into_inner()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("handle_get error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn handle_put(
    data: web::Data<LocalStorageService>,
    request: web::Json<PutRequest>,
) -> impl Responder {
    match data.put(request.into_inner()) {
        Ok(_) => HttpResponse::Ok().json(PutResponse {}),
        Err(e) => {
            log::error!("handle_put error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn handle_delete(
    data: web::Data<LocalStorageService>,
    request: web::Json<DeleteRequest>,
) -> impl Responder {
    match data.delete(request.into_inner()) {
        Ok(_) => HttpResponse::Ok().json(DeleteResponse {}),
        Err(e) => {
            log::error!("handle_delete error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn handle_list(
    data: web::Data<LocalStorageService>,
    _request: web::Json<ListRequest>,
) -> impl Responder {
    match data.list() {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("handle_list error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn handle_list_by_prefix(
    data: web::Data<LocalStorageService>,
    request: web::Json<ListByPrefixRequest>,
) -> impl Responder {
    match data.list_by_prefix(request.into_inner()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            log::error!("handle_list_by_prefix error: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let root_path_str = std::env::var("CK_ROOT_PATH").map_err(|e| {
        log::error!("CK_ROOT_PATH error: {:?}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;
    let root_path = std::path::Path::new(&root_path_str);
    let config_file_path = std::path::Path::new("config/runtime_config.toml");
    let runtime_config = RuntimeConfig::from_toml(root_path.join(config_file_path))
        .expect("failed to read runtime config");

    let ip = runtime_config
        .internal_endpoints
        .db_service
        .listen_address
        .ip();
    let port = runtime_config
        .internal_endpoints
        .db_service
        .listen_address
        .port();
    let db_path = root_path.join(runtime_config.storage.db_path);

    let storage_service =
        LocalStorageService::init(db_path, ip, port).expect("Failed to create storage service");
    storage_service.start().await
}
