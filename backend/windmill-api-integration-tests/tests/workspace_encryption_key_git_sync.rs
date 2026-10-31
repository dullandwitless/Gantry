/*!
 * Integration test for workspace encryption key rotation triggering git sync.
 *
 * Regression test for windmill-labs/windmill#9344 — re-encrypting all secret
 * variables on workspace key change must dispatch a git-sync job that carries
 * every re-encrypted variable plus the encryption_key entry, so repos with
 * Secrets sync enabled receive the new ciphertexts in one commit.
 *
 * Run with:
 * ```bash
 * cargo test --test workspace_encryption_key_git_sync
 * ```
 */

use serde_json::json;
use sqlx::{Pool, Postgres};
use std::time::Duration;

#[allow(unused_imports)]
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[allow(dead_code)]
async fn create_git_repo_resource(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
        VALUES ('test-workspace', 'u/test-user/test_git_repo', $1::jsonb, 'git_repository', '{}'::jsonb, 'test-user')
        ON CONFLICT (workspace_id, path) DO NOTHING
        "#,
    )
    .bind(json!({
        "url": "https://github.com/test/test.git",
        "branch": "main",
        "token": "test-token"
    }))
    .execute(db)
    .await?;
    Ok(())
}

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

#[allow(dead_code)]
async fn setup_git_sync_config(db: &Pool<Postgres>, sync_script_path: &str) -> anyhow::Result<()> {
    // Include Variable + Secret + Key so the encryption rotation has a reason
    // to push every re-encrypted variable. Anchor include_path to root so all
    // u/... and f/... paths pass the regex filter.
    let git_sync_config = json!({
        "include_type": ["variable", "secret", "key"],
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

/// Insert N secret variables, encrypting their values with the workspace's
/// current key so the re-encryption path can decrypt them.
#[allow(dead_code)]
async fn insert_secret_variables(db: &Pool<Postgres>, paths: &[&str]) -> anyhow::Result<()> {
    use windmill_common::variables::{build_crypt, encrypt};
    let mc = build_crypt(db, "test-workspace").await?;
    for path in paths {
        let plaintext = format!("secret-value-for-{path}");
        let encrypted = encrypt(&mc, &plaintext);
        sqlx::query!(
            r#"
            INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms, account)
            VALUES ($1, $2, $3, true, '', '{}'::jsonb, NULL)
            ON CONFLICT (workspace_id, path) DO UPDATE SET value = EXCLUDED.value
            "#,
            "test-workspace",
            path,
            encrypted,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

#[derive(Debug)]
#[allow(dead_code)]
struct DeploymentCallbackJob {
    id: uuid::Uuid,
    args: Option<serde_json::Value>,
}

/// Poll until at least `min_count` deployment-callback jobs exist for the
/// script path, or the timeout elapses. Returns whatever was found.
#[allow(dead_code)]
async fn wait_for_deployment_callbacks(
    db: &Pool<Postgres>,
    script_path: &str,
    min_count: usize,
    timeout: Duration,
) -> anyhow::Result<Vec<DeploymentCallbackJob>> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let rows = sqlx::query_as!(
            DeploymentCallbackJob,
            r#"
            SELECT j.id, j.args
            FROM v2_job j
            JOIN v2_job_queue q ON j.id = q.id
            WHERE j.runnable_path = $1
              AND j.kind = 'deploymentcallback'
              AND j.workspace_id = 'test-workspace'
            ORDER BY j.created_at DESC
            "#,
            script_path,
        )
        .fetch_all(db)
        .await?;
        if rows.len() >= min_count || tokio::time::Instant::now() >= deadline {
            return Ok(rows);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Stored repository tokens and webhook secrets are encrypted under the
/// workspace key but never synced, so a rotation has to carry them over even
/// when the caller skips re-encrypting variables.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_encryption_key_rotation_reencrypts_git_sync_secrets(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_common::variables::{build_crypt, crypt_from_key_with_suffix, decrypt, encrypt};
    initialize_tracing().await;

    create_folder(&db, "28103").await?;
    create_git_repo_resource(&db).await?;
    let sync_script_path = "f/28103/test_sync_script_git_secrets";
    create_sync_script(&db, sync_script_path).await?;
    setup_git_sync_config(&db, sync_script_path).await?;

    let mc = build_crypt(&db, "test-workspace").await?;
    sqlx::query(
        r#"
        UPDATE workspace_settings SET
            git_credentials = jsonb_build_array(jsonb_build_object(
                'token', $1::text, 'repo_identity', 'https://gitlab.example.com/grp/proj')),
            git_sync = jsonb_set(git_sync, '{repositories,0,auto_pull}', jsonb_build_object(
                'enabled', true, 'mode', 'webhook', 'webhook_id', 1, 'webhook_secret', $2::text))
        WHERE workspace_id = 'test-workspace'
        "#,
    )
    .bind(encrypt(&mc, "stored-token"))
    .bind(encrypt(&mc, "hook-secret"))
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    let new_key = "c".repeat(64);
    let resp = authed(client().post(format!("{base}/encryption_key")))
        .json(&json!({"new_key": new_key, "skip_reencrypt": true}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "set_encryption_key failed: {}",
        resp.text().await?
    );

    let (token, secret): (String, String) = sqlx::query_as(
        "SELECT git_credentials->0->>'token', git_sync#>>'{repositories,0,auto_pull,webhook_secret}'
         FROM workspace_settings WHERE workspace_id = 'test-workspace'",
    )
    .fetch_one(&db)
    .await?;
    let new_mc = crypt_from_key_with_suffix(&new_key, "");
    assert_eq!(decrypt(&new_mc, token)?, "stored-token");
    assert_eq!(decrypt(&new_mc, secret)?, "hook-secret");

    Ok(())
}
