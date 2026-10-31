//! Paths that hold a pool connection (an open transaction or an acquired connection) must not
//! wait for a second one from the same pool: a burst the size of the pool would have every
//! holder wait for another until the acquire timeout. On a one-connection pool any such second
//! checkout hangs.

use std::time::Duration;

use serde_json::json;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use windmill_common::{
    db::{AuthedRef, UserDB, UserDbWithAuthed},
    error::Error,
    jobs::JobPayload,
    worker::Connection,
};
use windmill_test_utils::*;

async fn one_connection_pool(db: &Pool<Postgres>) -> Pool<Postgres> {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with((*db.connect_options()).clone())
        .await
        .unwrap()
}

#[sqlx::test(fixtures("base"))]
async fn run_by_path_of_a_missing_script_uses_one_connection(db: Pool<Postgres>) {
    let one = one_connection_pool(&db).await;
    let groups = vec![];
    let folders = vec![];
    let authed = AuthedRef {
        email: "test2@windmill.dev",
        username: "test-user-2",
        is_admin: &false,
        is_operator: &false,
        groups: &groups,
        folders: &folders,
        scopes: &None,
        token_prefix: &None,
    };
    let lookup = windmill_common::get_latest_deployed_hash_for_path(
        Some(UserDbWithAuthed { authed: &authed, db: UserDB::new(one.clone()) }),
        one.clone(),
        "test-workspace",
        "f/missing/script",
    );
    let res = tokio::time::timeout(Duration::from_secs(15), lookup)
        .await
        .unwrap();
    assert!(matches!(res, Err(Error::NotFound(_))), "{:?}", res.err());
}

#[sqlx::test(fixtures("base"))]
async fn for_loop_flow_runs_on_one_connection(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let flow = serde_json::from_value(json!({
        "modules": [{
            "id": "loop",
            "value": {
                "type": "forloopflow",
                "iterator": { "type": "static", "value": [1, 2] },
                "skip_failures": false,
                "parallel": false,
                "modules": [{ "id": "inner", "value": { "type": "identity" } }]
            }
        }]
    }))?;
    let job = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .push(&db)
        .await;

    let one = one_connection_pool(&db).await;
    let db2 = db.clone();
    in_test_worker(
        Connection::Sql(one),
        async move {
            while sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM v2_job_completed WHERE id = $1)",
            )
            .bind(job)
            .fetch_one(&db2)
            .await
            .unwrap()
                == false
            {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        },
        server.addr.port(),
    )
    .await;

    let status: String =
        sqlx::query_scalar("SELECT status::text FROM v2_job_completed WHERE id = $1")
            .bind(job)
            .fetch_one(&db)
            .await?;
    assert_eq!(status, "success");
    Ok(())
}
