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

#[cfg(feature = "oauth2")]
use crate::OAUTH_CLIENTS;
use windmill_common::error;
use windmill_common::oauth2::*;

use crate::db::DB;
use std::str;

pub fn global_service() -> Router {
    let router = Router::new().route("/list_connects", get(list_connects));
    #[cfg(feature = "oauth2")]
    let router = router.merge(crate::oauth_login::routes());
    router
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
    base_url: &str,
    oauths_from_config: Option<HashMap<String, OAuthClient>>,
    _db: &DB,
) -> anyhow::Result<AllClients> {
    // Sign-in providers: specs/sso-oauth-login.md. Resource connects are not implemented.
    Ok(AllClients {
        logins: crate::oauth_login::login_clients(
            &oauths_from_config.unwrap_or_default(),
            base_url,
        ),
        connects: HashMap::default(),
        slack: None,
        slack_signing_secret: None,
    })
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
    // Stub: not implemented.
    return Ok(axum::Json(vec![]));
}

pub async fn _refresh_token(
    _path: &str,
    _w_id: &str,
    _id: i32,
    _db: &DB,
    _force: bool,
) -> error::Result<String> {
    // Stub: not implemented.
    Err(error::Error::BadRequest(
        "Not implemented in Gantry".to_string(),
    ))
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
