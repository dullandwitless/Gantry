use sqlx::Postgres;

use crate::{error::Result, DB};

pub async fn get_disable_stats_setting(_db: &DB) -> bool {
    // stats details are closed source

    false
}

pub async fn schedule_stats(_db: &DB, _http_client: &reqwest::Client) -> () {
    // stats details are closed source
}

pub enum SendStatsReason {
    Manual,
    Schedule,
    OnStart,
}

pub async fn send_stats(
    _http_client: &reqwest::Client,
    _db: &DB,
    _reason: SendStatsReason,
    _minimal: bool,
) -> Result<()> {
    // stats details are closed source
    Ok(())
}

pub struct ActiveUserUsage {
    pub author_count: Option<i32>,
    pub operator_count: Option<i32>,
}

pub async fn get_user_usage<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    _db: E,
) -> Result<ActiveUserUsage> {
    let usage = ActiveUserUsage { author_count: None, operator_count: None };
    Ok(usage)
}

#[derive(serde::Serialize)]
pub struct Stats {}

pub async fn get_stats_payload(
    _db: &DB,
    _reason: &SendStatsReason,
    _minimal: bool,
) -> Result<Stats> {
    // stats details are closed source
    Ok(Stats {})
}

pub fn sign_stats(_json: &str) -> String {
    // stats details are closed source
    String::new()
}
