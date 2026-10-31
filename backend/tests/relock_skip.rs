use sqlx::{Pool, Postgres};
use tokio_stream::StreamExt;
use windmill_api_client::types::NewScript;
use windmill_test_utils::*;

mod relock_skip {
    use super::*;

    fn quick_ns(
        content: &str,
        language: windmill_api_client::types::ScriptLang,
        path: &str,
        lock: Option<String>,
        parent_hash: Option<String>,
    ) -> NewScript {
        NewScript {
            draft_only: None,
            content: content.into(),
            language,
            lock,
            parent_hash,
            path: path.into(),
            concurrent_limit: None,
            concurrency_time_window_s: None,
            cache_ttl: None,
            dedicated_worker: None,
            description: "".to_string(),
            envs: vec![],
            is_template: None,
            kind: None,
            summary: "".to_string(),
            tag: None,
            schema: std::collections::HashMap::new(),
            ws_error_handler_muted: Some(false),
            priority: None,
            delete_after_secs: None,
            timeout: None,
            restart_unless_cancelled: None,
            deployment_message: None,
            concurrency_key: None,
            visible_to_runner_only: None,
            auto_kind: None,
            codebase: None,
            has_preprocessor: None,
            on_behalf_of_email: None,
            assets: vec![],
            modules: None,
        }
    }

    async fn init(db: Pool<Postgres>) -> (windmill_api_client::Client, u16, ApiServer) {
        init_client(db).await
    }

    /// Counts occurrences of a pattern in job logs for all jobs created after a given time
    async fn count_pattern_in_job_logs(
        db: &Pool<Postgres>,
        pattern: &str,
        after: chrono::DateTime<chrono::Utc>,
    ) -> i64 {
        let logs = sqlx::query_scalar!("SELECT logs FROM job_logs WHERE created_at > $1", after)
            .fetch_all(db)
            .await
            .unwrap();

        logs.iter()
            .filter_map(|l| l.as_ref())
            .map(|l| l.matches(pattern).count() as i64)
            .sum()
    }

    /// Waits for exactly N jobs to complete. Returns the timestamp before waiting.
    async fn wait_for_jobs(
        completed: &mut (impl futures::Stream<Item = uuid::Uuid> + Unpin),
        count: usize,
    ) -> chrono::DateTime<chrono::Utc> {
        let before = chrono::Utc::now();
        for _ in 0..count {
            completed.next().await;
        }
        before
    }

    /// Waits for at least N jobs to complete, then drains any additional jobs
    /// that complete within a short timeout. Returns the timestamp before waiting.
    async fn wait_for_jobs_ge(
        completed: &mut (impl futures::Stream<Item = uuid::Uuid> + Unpin),
        min_count: usize,
    ) -> chrono::DateTime<chrono::Utc> {
        let before = chrono::Utc::now();
        for _ in 0..min_count {
            completed.next().await;
        }
        // Drain any additional jobs that complete within 5 seconds
        loop {
            match tokio::time::timeout(std::time::Duration::from_secs(1), completed.next()).await {
                Ok(Some(_)) => continue,
                _ => break,
            }
        }
        before
    }

}
