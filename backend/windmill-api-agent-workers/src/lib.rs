
/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_common::DB;

use axum::Router;

pub fn global_service(_job_completed_tx: windmill_worker::JobCompletedSender) -> Router {
    Router::new()
}

pub fn workspaced_service(
    db: DB,
    _base_internal_url: String,
) -> (
    Router,
    Vec<tokio::task::JoinHandle<()>>,
    Option<windmill_worker::JobCompletedSender>,
) {
    use windmill_common::worker::Connection;
    use windmill_worker::JobCompletedSender;

    let (job_completed_tx, _job_completed_rx) =
        JobCompletedSender::new(&Connection::Sql(db.clone()), 10);

    let router = Router::new();

    (router, vec![], Some(job_completed_tx))
}

pub struct AgentCache {}

impl AgentCache {
    pub fn new() -> Self {
        AgentCache {}
    }

    pub async fn extract_worker_name(
        &self,
        _token: &str,
        _db: &windmill_common::DB,
    ) -> Option<String> {
        None
    }
}
