use crate::ee_oss::LicensePlan::Community;
use serde::Deserialize;
use std::sync::atomic::AtomicBool;

lazy_static::lazy_static! {
  pub static ref LICENSE_KEY_VALID: AtomicBool = AtomicBool::new(true);
  pub static ref LICENSE_KEY_ID: arc_swap::ArcSwap<String> = arc_swap::ArcSwap::from_pointee("".to_string());
  pub static ref LICENSE_KEY: arc_swap::ArcSwap<String> = arc_swap::ArcSwap::from_pointee("".to_string());
  pub static ref LICENSE_OFFLINE_METADATA: arc_swap::ArcSwap<Option<OfflineMetadata>> = arc_swap::ArcSwap::from_pointee(None);
  pub static ref LICENSE_OFFLINE_OVER_CU_CAP: AtomicBool = AtomicBool::new(false);
  pub static ref LICENSE_OFFLINE_OVER_SEAT_CAP: AtomicBool = AtomicBool::new(false);
  pub static ref LICENSE_OFFLINE_LAST_STATUS: arc_swap::ArcSwap<Option<OfflineCapStatus>> = arc_swap::ArcSwap::from_pointee(None);
  pub static ref LICENSE_OFFLINE_LAST_CHECKED_AT: arc_swap::ArcSwap<Option<chrono::DateTime<chrono::Utc>>> = arc_swap::ArcSwap::from_pointee(None);
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct OfflineMetadata {
    pub v: u32,
    pub kind: String,
    pub hash: String,
    pub seats: i64,
    pub cu_limit: f64,
}

impl OfflineMetadata {
    pub fn is_offline(&self) -> bool {
        self.kind == "offline"
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct OfflineCapStatus {
    pub seats_used: f64,
    pub seats_cap: i64,
    pub author_count: i64,
    pub operator_count: i64,
    pub current_cu: f64,
    pub cu_cap: f64,
    pub cu_over_cap: bool,
}

#[derive(PartialEq, Eq)]
pub enum LicensePlan {
    Community,
    Pro,
    Enterprise,
}

pub async fn get_license_plan() -> LicensePlan {
    // Implementation is not open source
    return Community;
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum CriticalErrorChannel {
    Email { email: String },
    Slack { slack_channel: String },
    Teams { teams_channel: TeamsChannel },
}

#[derive(Deserialize)]
pub struct TeamsChannel {
    pub team_id: String,
    pub team_name: String,
    pub channel_id: String,
    pub channel_name: String,
}

pub enum CriticalAlertKind {
}

