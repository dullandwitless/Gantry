/*!
 * Integration tests for workspace dependencies git sync.
 *
 * These tests verify that creating, archiving, and deleting workspace dependencies
 * triggers deployment callback jobs with the correct arguments for git sync.
 *
 * Run with enterprise features:
 * ```bash
 * cargo test --test workspace_dependencies_git_sync --features enterprise,private
 * ```
 */

use serde_json::json;
use sqlx::{Pool, Postgres};
use std::time::Duration;

#[allow(unused_imports)]
use windmill_test_utils::*;

/// Row shape for querying deployment callback jobs from v2_job_queue
#[derive(Debug)]
#[allow(dead_code)]
struct DeploymentCallbackJob {
    id: uuid::Uuid,
    runnable_path: Option<String>,
    args: Option<serde_json::Value>,
    kind: String,
}

/// Poll for deployment callback jobs in the queue for a given script path
#[allow(dead_code)]
async fn get_deployment_callback_jobs(
    db: &Pool<Postgres>,
    script_path: &str,
    timeout: Duration,
) -> anyhow::Result<Vec<DeploymentCallbackJob>> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let rows = sqlx::query_as!(
            DeploymentCallbackJob,
            r#"
            SELECT j.id, j.runnable_path, j.args, j.kind::text AS "kind!"
            FROM v2_job j
            JOIN v2_job_queue q ON j.id = q.id
            WHERE j.runnable_path = $1
              AND j.kind = 'deploymentcallback'
            ORDER BY j.created_at DESC
            "#,
            script_path,
        )
        .fetch_all(db)
        .await?;

        if !rows.is_empty() {
            return Ok(rows);
        }

        if tokio::time::Instant::now() >= deadline {
            // Return empty if timeout - caller will handle assertion
            return Ok(vec![]);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Configure git sync for the test workspace with workspace dependencies enabled
#[allow(dead_code)]
async fn setup_git_sync_config(db: &Pool<Postgres>, sync_script_path: &str) -> anyhow::Result<()> {
    let git_sync_config = json!({
        "include_type": ["workspacedependencies"],
        "include_path": ["**"],
        "repositories": [{
            "script_path": sync_script_path,
            "git_repo_resource_path": "$res:u/test-user/test_git_repo",
            "use_individual_branch": false,
            "group_by_folder": false
        }]
    });

    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        git_sync_config,
        "test-workspace"
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Create a git repository resource at an arbitrary path.
#[allow(dead_code)]
async fn create_git_repo_resource_at(db: &Pool<Postgres>, path: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
        VALUES ('test-workspace', $2, $1::jsonb, 'git_repository', '{}'::jsonb, 'test-user')
        ON CONFLICT (workspace_id, path) DO NOTHING
        "#,
    )
    .bind(json!({
        "url": format!("https://github.com/test/{}.git", path.rsplit('/').next().unwrap_or("test")),
        "branch": "main",
        "token": "test-token"
    }))
    .bind(path)
    .execute(db)
    .await?;
    Ok(())
}

/// Create a git repository resource for testing
#[allow(dead_code)]
async fn create_git_repo_resource(db: &Pool<Postgres>) -> anyhow::Result<()> {
    create_git_repo_resource_at(db, "u/test-user/test_git_repo").await
}

/// Create a dummy sync script for testing (with version >= 28103 for debouncing support)
#[allow(dead_code)]
async fn create_sync_script(db: &Pool<Postgres>, path: &str) -> anyhow::Result<i64> {
    let hash: i64 = rand::random::<i64>().unsigned_abs() as i64;
    sqlx::query(
        r#"
        INSERT INTO script (workspace_id, hash, path, summary, description, content,
                  created_by, language, kind, lock)
        VALUES ('test-workspace', $1, $2, 'sync script', '',
                'export function main(items: any[]) { return { synced: items.length }; }',
                'test-user', 'bun', 'script', '')
        "#,
    )
    .bind(hash)
    .bind(path)
    .execute(db)
    .await?;
    Ok(hash)
}

/// Create a folder for the versioned script path
#[allow(dead_code)]
async fn create_folder(db: &Pool<Postgres>, name: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
        VALUES ('test-workspace', $1, $1, ARRAY['u/test-user'], '{}'::jsonb, 'test-user')
        ON CONFLICT (workspace_id, name) DO NOTHING
        "#,
    )
    .bind(name)
    .execute(db)
    .await?;
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

// ============================================================================
// Promotion-mode debounce-key tests
// ============================================================================

/// Configure git sync in promotion mode (one branch per object), with explicit
/// include_type and include_path lists so script deploys fire the callback.
#[allow(dead_code)]
async fn setup_promotion_git_sync_config(
    db: &Pool<Postgres>,
    sync_script_path: &str,
    group_by_folder: bool,
) -> anyhow::Result<()> {
    let git_sync_config = json!({
        "include_type": ["script"],
        "include_path": ["**"],
        "repositories": [{
            "script_path": sync_script_path,
            "git_repo_resource_path": "$res:u/test-user/test_git_repo",
            "use_individual_branch": true,
            "group_by_folder": group_by_folder
        }]
    });

    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        git_sync_config,
        "test-workspace"
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Create a script via the API (triggering handle_deployment_metadata).
#[allow(dead_code)]
async fn create_test_script(
    client: &windmill_api_client::Client,
    path: &str,
) -> anyhow::Result<()> {
    let resp = client
        .client()
        .post(format!(
            "{}/w/test-workspace/scripts/create",
            client.baseurl()
        ))
        .json(&json!({
            "path": path,
            "summary": "",
            "description": "",
            // bash has no lock step, so handle_deployment_metadata runs
            "content": "echo hi",
            "language": "bash"
        }))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    anyhow::ensure!(
        status.is_success(),
        "failed to create script {}: {} {}",
        path,
        status,
        body
    );
    Ok(())
}

/// Poll the `debounce_key` table until `expected` appears, or timeout.
#[allow(dead_code)]
async fn wait_for_debounce_key(
    db: &Pool<Postgres>,
    expected: &str,
    timeout: Duration,
) -> anyhow::Result<Vec<String>> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let keys: Vec<String> =
            sqlx::query_scalar!("SELECT key FROM debounce_key WHERE key LIKE 'git_sync:%'")
                .fetch_all(db)
                .await?;
        if keys.iter().any(|k| k == expected) {
            return Ok(keys);
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(keys);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Poll until `expected` callbacks for `script_path` are queued, or time out.
#[allow(dead_code)]
async fn wait_for_callback_jobs(
    db: &Pool<Postgres>,
    script_path: &str,
    expected: usize,
    timeout: Duration,
) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let jobs =
            get_deployment_callback_jobs(db, script_path, Duration::from_millis(200)).await?;
        if jobs.len() >= expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "timed out waiting for {expected} deployment callbacks on {script_path}, got {}",
                jobs.len()
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// The concurrency key each queued deployment callback was pushed with, read
/// back through the runnable-settings handle the push stored it under.
#[allow(dead_code)]
async fn get_concurrency_keys(
    db: &Pool<Postgres>,
    script_path: &str,
) -> anyhow::Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT COALESCE(cs.concurrency_key, '<no concurrency settings row>')
        FROM v2_job j
        JOIN v2_job_queue q ON q.id = j.id
        LEFT JOIN runnable_settings rs ON rs.hash = q.runnable_settings_handle
        LEFT JOIN concurrency_settings cs ON cs.hash = rs.concurrency_settings
        WHERE j.runnable_path = $1 AND j.kind = 'deploymentcallback'
        "#,
    )
    .bind(script_path)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(k,)| k).collect())
}

/// Create a second git repository resource for multi-repo tests.
#[allow(dead_code)]
async fn create_second_git_repo_resource(db: &Pool<Postgres>) -> anyhow::Result<()> {
    create_git_repo_resource_at(db, "u/test-user/test_git_repo_2").await
}

/// Configure git sync with TWO promotion-mode repositories pointing at distinct
/// git repo resources. Both repos use the same sync script and the same item
/// filters — they only differ in the repo they target.
#[allow(dead_code)]
async fn setup_two_promotion_repos_config(
    db: &Pool<Postgres>,
    sync_script_path: &str,
    group_by_folder: bool,
) -> anyhow::Result<()> {
    let git_sync_config = json!({
        "include_type": ["script"],
        "include_path": ["**"],
        "repositories": [
            {
                "script_path": sync_script_path,
                "git_repo_resource_path": "$res:u/test-user/test_git_repo",
                "use_individual_branch": true,
                "group_by_folder": group_by_folder
            },
            {
                "script_path": sync_script_path,
                "git_repo_resource_path": "$res:u/test-user/test_git_repo_2",
                "use_individual_branch": true,
                "group_by_folder": group_by_folder
            }
        ]
    });

    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        git_sync_config,
        "test-workspace"
    )
    .execute(db)
    .await?;

    Ok(())
}

