/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, fmt::Debug};

use axum::{routing::get, Json, Router};
use hmac::Mac;

#[cfg(feature = "oauth2")]
use itertools::Itertools;
use serde::{Deserialize, Serialize};
#[cfg(feature = "oauth2")]
use windmill_common::more_serde::maybe_number_opt;
#[cfg(feature = "oauth2")]
use windmill_oauth::{helpers, AccessToken, RefreshToken, Scope};

#[cfg(feature = "oauth2")]
use crate::OAUTH_CLIENTS;
use windmill_common::error;
use windmill_common::oauth2::*;

use crate::db::DB;
use std::str;

pub fn global_service() -> Router {
    Router::new()
        .route("/list_logins", get(list_logins))
        .route("/list_connects", get(list_connects))
}

pub fn workspaced_service() -> Router {
    Router::new()
}

pub async fn workspace_connect_slack() -> Result<http::status::StatusCode, error::Error> {
    Err(error::Error::BadRequest(
        "Slack only available on enterprise".to_string(),
    ))
}

pub async fn connect_slack_instance() -> Result<http::status::StatusCode, error::Error> {
    Err(error::Error::BadRequest(
        "Slack only available on enterprise".to_string(),
    ))
}

#[cfg(feature = "oauth2")]
pub use windmill_oauth::{AllClients, BasicClientsMap, ClientWithScopes};

pub use windmill_oauth::{OAuthClient, OAuthConfig};

#[cfg(feature = "oauth2")]
pub async fn build_oauth_clients(
    _base_url: &str,
    _oauths_from_config: Option<HashMap<String, OAuthClient>>,
    _db: &DB,
) -> anyhow::Result<AllClients> {
    // Implementation is not open source
    return Ok(AllClients {
        logins: HashMap::default(),
        connects: HashMap::default(),
        slack: None,
        slack_signing_secret: None,
    });
}

#[cfg(feature = "oauth2")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    access_token: AccessToken,
    #[serde(deserialize_with = "maybe_number_opt")]
    #[serde(default)]
    expires_in: Option<u64>,
    refresh_token: Option<RefreshToken>,
    #[serde(deserialize_with = "helpers::deserialize_space_delimited_vec")]
    #[serde(serialize_with = "helpers::serialize_space_delimited_vec")]
    #[serde(default)]
    scope: Option<Vec<Scope>>,
}

#[derive(Serialize)]
struct Logins {
    oauth: Vec<String>,
    saml: Option<String>,
    auto_login: Option<String>,
}
async fn list_logins() -> error::JsonResult<Logins> {
    // Implementation is not open source
    return Ok(Json(Logins { oauth: vec![], saml: None, auto_login: None }));
}

#[allow(unused)]
#[cfg(feature = "oauth2")]
async fn list_connects() -> error::JsonResult<Vec<String>> {
    Ok(Json(
        (&OAUTH_CLIENTS.load().connects)
            .keys()
            .map(|x| x.to_owned())
            .collect_vec(),
    ))
}

#[allow(unused)]
#[cfg(not(feature = "oauth2"))]
async fn list_connects() -> windmill_common::error::JsonResult<Vec<String>> {
    // Implementation is not open source
    return Ok(axum::Json(vec![]));
}

pub async fn _refresh_token(
    _path: &str,
    _w_id: &str,
    _id: i32,
    _db: &DB,
    _force: bool,
) -> error::Result<String> {
    // Implementation is not open source
    Err(error::Error::BadRequest(
        "Not implemented in Windmill's Open Source repository".to_string(),
    ))
}

pub async fn check_nb_of_user(db: &DB) -> error::Result<()> {
    let nb_users_sso =
        sqlx::query_scalar!("SELECT COUNT(*) FROM password WHERE login_type != 'password'",)
            .fetch_one(db)
            .await?;
    if nb_users_sso.unwrap_or(0) >= 10 {
        return Err(error::Error::BadRequest(
            "You have reached the maximum number of oauth users accounts (10) without an enterprise license"
                .to_string(),
        ));
    }

    let nb_users = sqlx::query_scalar!("SELECT COUNT(*) FROM password",)
        .fetch_one(db)
        .await?;
    if nb_users.unwrap_or(0) >= 50 {
        return Err(error::Error::BadRequest(
            "You have reached the maximum number of accounts (50) without an enterprise license"
                .to_string(),
        ));
    }
    return Ok(());
}

#[derive(Clone, Debug)]
pub struct SlackVerifier {
    mac: HmacSha256,
}
impl SlackVerifier {
    pub fn new<S: AsRef<[u8]>>(secret: S) -> anyhow::Result<SlackVerifier> {
        HmacSha256::new_from_slice(secret.as_ref())
            .map(|mac| SlackVerifier { mac })
            .map_err(|_| anyhow::anyhow!("invalid secret"))
    }

    pub fn verify(&self, ts: &str, body: &str, exp_sig: &str) -> anyhow::Result<()> {
        let basestring = format!("v0:{}:{}", ts, body);
        let mut mac = self.mac.clone();
        mac.update(basestring.as_bytes());
        let sig = format!("v0={}", hex::encode(mac.finalize().into_bytes()));
        if sig != exp_sig {
            Err(anyhow::anyhow!("signature mismatch"))?;
        }
        Ok(())
    }
}
