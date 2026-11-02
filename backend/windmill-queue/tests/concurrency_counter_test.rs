//! Tests for concurrency-limit admission (`windmill_queue::jobs_oss`), per
//! `specs/concurrency-limits.md`.
//!
//! The structural property under test is the **never-over-admit invariant**:
//! across N concurrent admission attempts on a single concurrency key with
//! limit L, the number of admits must never exceed L.
//!
//! Run with:
//!   cargo test -p windmill-queue --test concurrency_counter_test \
//!       -- --nocapture

use chrono::{DateTime, Duration, Utc};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_queue::jobs_oss::{
    admit, clear_deferred_marker, defer, wake_next_deferred, Admission, DEFERRED_MARKER,
    DEFERRED_RECHECK_S,
};

const KEY: &str = "ws/script/f/limited";

/// Queues a job on `KEY`, as the pull query leaves it: running when `running`.
async fn queue_job(db: &Pool<Postgres>, running: bool, scheduled_in_s: i64) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO v2_job (id, workspace_id, tag) VALUES ($1, 'ws', 'other')")
        .bind(id)
        .execute(db)
        .await
        .expect("seed v2_job");
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, tag, running, started_at, scheduled_for)
         VALUES ($1, 'ws', 'other', $2, CASE WHEN $2 THEN now() END,
                 now() + make_interval(secs => $3))",
    )
    .bind(id)
    .bind(running)
    .bind(scheduled_in_s as f64)
    .execute(db)
    .await
    .expect("seed v2_job_queue");
    sqlx::query(
        "WITH c AS (
             INSERT INTO concurrency_counter (concurrency_id, job_uuids) VALUES ($1, '{}'::jsonb)
             ON CONFLICT DO NOTHING
         )
         INSERT INTO concurrency_key (key, job_id) VALUES ($1, $2)",
    )
    .bind(KEY)
    .bind(id)
    .execute(db)
    .await
    .expect("seed concurrency_key");
    id
}

/// Records a job of `KEY` that started `started_s_ago` and completed `ended_s_ago` seconds ago.
async fn completed_job(db: &Pool<Postgres>, started_s_ago: i64, ended_s_ago: i64) -> DateTime<Utc> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO v2_job (id, workspace_id, tag) VALUES ($1, 'ws', 'other')")
        .bind(id)
        .execute(db)
        .await
        .expect("seed v2_job");
    let started_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO v2_job_completed (id, workspace_id, started_at, completed_at, duration_ms, status)
         VALUES ($1, 'ws', now() - make_interval(secs => $2), now() - make_interval(secs => $3),
                 0, 'success')
         RETURNING started_at",
    )
    .bind(id)
    .bind(started_s_ago as f64)
    .bind(ended_s_ago as f64)
    .fetch_one(db)
    .await
    .expect("seed v2_job_completed");
    sqlx::query(
        "INSERT INTO concurrency_key (key, job_id, ended_at)
         VALUES ($1, $2, now() - make_interval(secs => $3))",
    )
    .bind(KEY)
    .bind(id)
    .bind(ended_s_ago as f64)
    .execute(db)
    .await
    .expect("seed concurrency_key");
    started_at
}

async fn try_admit(db: &Pool<Postgres>, job: Uuid, limit: i32, window_s: i32) -> Admission {
    let mut tx = db.begin().await.expect("begin");
    let admission = admit(&mut tx, job, KEY, limit, window_s)
        .await
        .expect("admit");
    tx.commit().await.expect("commit");
    admission
}

async fn admitted_count(db: &Pool<Postgres>) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM concurrency_counter, jsonb_object_keys(job_uuids)
         WHERE concurrency_id = $1",
    )
    .bind(KEY)
    .fetch_one(db)
    .await
    .expect("count admitted")
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_admissions_never_exceed_the_limit(db: Pool<Postgres>) {
    const LIMIT: i32 = 3;
    let mut jobs = Vec::new();
    for _ in 0..20 {
        jobs.push(queue_job(&db, true, 0).await);
    }

    let attempts = jobs.into_iter().map(|job| {
        let db = db.clone();
        tokio::spawn(async move { try_admit(&db, job, LIMIT, 0).await })
    });
    let mut admitted = 0;
    for attempt in attempts {
        if attempt.await.expect("join") == Admission::Admitted {
            admitted += 1;
        }
    }

    assert_eq!(admitted, LIMIT);
    assert_eq!(admitted_count(&db).await, i64::from(LIMIT));
}

#[sqlx::test(migrations = "../migrations")]
async fn a_job_holding_a_slot_is_admitted_again(db: Pool<Postgres>) {
    let flow = queue_job(&db, true, 0).await;
    assert_eq!(try_admit(&db, flow, 1, 0).await, Admission::Admitted);
    assert_eq!(try_admit(&db, flow, 1, 0).await, Admission::Admitted);
    assert_eq!(admitted_count(&db).await, 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn running_jobs_defer_until_the_recheck(db: Pool<Postgres>) {
    let first = queue_job(&db, true, 0).await;
    let second = queue_job(&db, true, 0).await;
    assert_eq!(try_admit(&db, first, 1, 0).await, Admission::Admitted);

    let before = Utc::now();
    let Admission::Deferred { until } = try_admit(&db, second, 1, 0).await else {
        panic!("the second job must wait for the first");
    };
    let wait = until - before;
    assert!(
        wait > Duration::seconds(i64::from(DEFERRED_RECHECK_S) - 5)
            && wait <= Duration::seconds(i64::from(DEFERRED_RECHECK_S) + 5),
        "deferred by {wait}, expected about {DEFERRED_RECHECK_S}s"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_window_counts_jobs_started_within_it(db: Pool<Postgres>) {
    // Limit 2 per 60s: one completed run started 50s ago, one 20s ago.
    completed_job(&db, 50, 45).await;
    let newest = completed_job(&db, 20, 15).await;
    let job = queue_job(&db, true, 0).await;

    // With limit 1 both runs have to leave the window: the newer one leaves at its start + 60s.
    let Admission::Deferred { until } = try_admit(&db, job, 1, 60).await else {
        panic!("limit 1 is used up");
    };
    let expected = newest + Duration::seconds(60);
    assert!(
        (until - expected).num_milliseconds().abs() < 1000,
        "deferred until {until}, expected {expected}"
    );

    // Both runs fill limit 2; limit 3 leaves a slot.
    assert!(matches!(
        try_admit(&db, job, 2, 60).await,
        Admission::Deferred { .. }
    ));
    assert_eq!(try_admit(&db, job, 3, 60).await, Admission::Admitted);
}

#[sqlx::test(migrations = "../migrations")]
async fn runs_outside_the_window_or_without_one_do_not_count(db: Pool<Postgres>) {
    completed_job(&db, 120, 110).await;
    completed_job(&db, 5, 1).await;
    let job = queue_job(&db, true, 0).await;

    // Without a window only running jobs count.
    assert_eq!(try_admit(&db, job, 1, 0).await, Admission::Admitted);

    // With a 60s window only the run that started 5s ago counts.
    let other = queue_job(&db, true, 0).await;
    assert_eq!(try_admit(&db, other, 3, 60).await, Admission::Admitted);
}

#[sqlx::test(migrations = "../migrations")]
async fn ids_whose_job_left_the_queue_do_not_count(db: Pool<Postgres>) {
    let gone = queue_job(&db, true, 0).await;
    assert_eq!(try_admit(&db, gone, 1, 0).await, Admission::Admitted);
    sqlx::query("DELETE FROM v2_job_queue WHERE id = $1")
        .bind(gone)
        .execute(&db)
        .await
        .expect("purge");

    let job = queue_job(&db, true, 0).await;
    assert_eq!(try_admit(&db, job, 1, 0).await, Admission::Admitted);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_missing_counter_is_recreated(db: Pool<Postgres>) {
    let job = queue_job(&db, true, 0).await;
    sqlx::query("DELETE FROM concurrency_counter")
        .execute(&db)
        .await
        .expect("drop counters");
    assert_eq!(try_admit(&db, job, 1, 0).await, Admission::Admitted);
    assert_eq!(admitted_count(&db).await, 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn defer_parks_the_job_and_admission_clears_the_mark(db: Pool<Postgres>) {
    let job = queue_job(&db, true, 0).await;
    let until = Utc::now() + Duration::seconds(30);

    let mut tx = db.begin().await.expect("begin");
    defer(&mut tx, job, until).await.expect("defer");
    tx.commit().await.expect("commit");

    let (running, started, worker, deferred): (bool, Option<DateTime<Utc>>, Option<String>, bool) =
        sqlx::query_as(
            "SELECT running, started_at, worker, coalesce(extras ? $2, false)
             FROM v2_job_queue WHERE id = $1",
        )
        .bind(job)
        .bind(DEFERRED_MARKER)
        .fetch_one(&db)
        .await
        .expect("read job");
    assert!(!running && started.is_none() && worker.is_none() && deferred);

    let mut tx = db.begin().await.expect("begin");
    clear_deferred_marker(&mut tx, job).await.expect("clear");
    tx.commit().await.expect("commit");
    let extras: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT extras FROM v2_job_queue WHERE id = $1")
            .bind(job)
            .fetch_one(&db)
            .await
            .expect("read extras");
    assert_eq!(extras, None);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_release_wakes_only_the_longest_parked_deferred_job(db: Pool<Postgres>) {
    // Scheduled for later by its caller, not by the limit: never woken.
    queue_job(&db, false, 10).await;
    let parked_first = queue_job(&db, true, 0).await;
    let parked_later = queue_job(&db, true, 0).await;
    for (job, in_s) in [(parked_first, 20), (parked_later, 40)] {
        let mut tx = db.begin().await.expect("begin");
        defer(&mut tx, job, Utc::now() + Duration::seconds(in_s))
            .await
            .expect("defer");
        tx.commit().await.expect("commit");
    }

    let mut conn = db.acquire().await.expect("acquire");
    wake_next_deferred(&mut conn, KEY).await.expect("wake");

    let due: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM v2_job_queue WHERE scheduled_for <= now() ORDER BY id")
            .fetch_all(&db)
            .await
            .expect("read due jobs");
    assert_eq!(due, vec![parked_first]);
}
