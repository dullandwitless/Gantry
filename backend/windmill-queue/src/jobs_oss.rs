//! Concurrency limits: whether a job may start under the limit of its concurrency key.
//!
//! Clean-room implementation of `specs/concurrency-limits.md`.
//! Attestation: written without reading Windmill's frontend source or proprietary code.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;
use windmill_common::error::{Error, Result};

/// Seconds a job refused because its key's slots are all held by running jobs waits before it
/// is checked again. Their end cannot be predicted; completion normally wakes the next
/// waiting job sooner (`wake_next_deferred`).
pub const DEFERRED_RECHECK_S: i32 = 60;

/// Key in `v2_job_queue.extras` marking a job parked by its concurrency limit, so that a
/// completion wakes only jobs the limit parked, never ones scheduled for later on purpose.
pub const DEFERRED_MARKER: &str = "concurrency_deferred";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// The job holds a slot of its key and may run.
    Admitted,
    /// The key is at its limit; a slot can free at the earliest at `until`.
    Deferred { until: DateTime<Utc> },
}

/// Decides whether `job_id` may start under `limit` jobs per `window_s` seconds for `key`,
/// and records the admission.
///
/// A job occupies a slot while it is admitted and still queued, and once completed until
/// `window_s` seconds after it started. Locking the key's counter row serializes concurrent
/// callers, so a key never has more than `limit` admitted jobs. Admitting a job that already
/// holds a slot (a flow pulled again) is a no-op.
///
/// The caller owns the transaction: commit it to keep the admission, and re-queue the job
/// in the same transaction when it is deferred.
pub async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    key: &str,
    limit: i32,
    window_s: i32,
) -> Result<Admission> {
    let job = job_id.hyphenated().to_string();
    let window_s = window_s.max(0);

    if lock_counter(tx, key, &job).await? {
        return Ok(Admission::Admitted);
    }

    // Admitted jobs still in the queue. Ids whose job left the queue without completing are
    // ignored here and removed by the monitor's orphan cleanup.
    let running: i64 = sqlx::query_scalar(
        "SELECT count(*)
         FROM concurrency_counter c
         CROSS JOIN LATERAL jsonb_object_keys(c.job_uuids) AS k(id)
         JOIN v2_job_queue q ON q.id = k.id::uuid
         WHERE c.concurrency_id = $1",
    )
    .bind(key)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| db_err("count running jobs", key, e))?;

    if running >= i64::from(limit) {
        let until = sqlx::query_scalar("SELECT now() + make_interval(secs => $1)")
            .bind(f64::from(DEFERRED_RECHECK_S))
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| db_err("compute recheck time", key, e))?;
        return Ok(Admission::Deferred { until });
    }

    // Completed jobs still inside the window. A job that started within the window also
    // ended within it, so the `ended_at` filter narrows the scan through the key's index.
    let recent: i64 = if window_s > 0 {
        sqlx::query_scalar(
            "SELECT count(*)
             FROM concurrency_key ck
             JOIN v2_job_completed j ON j.id = ck.job_id
             WHERE ck.key = $1
               AND ck.ended_at >= now() - make_interval(secs => $2)
               AND j.started_at >= now() - make_interval(secs => $2)",
        )
        .bind(key)
        .bind(f64::from(window_s))
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| db_err("count jobs in the window", key, e))?
    } else {
        0
    };

    let excess = running + recent - i64::from(limit);
    if excess < 0 {
        sqlx::query(
            "UPDATE concurrency_counter
             SET job_uuids = job_uuids || jsonb_build_object($2::text, now())
             WHERE concurrency_id = $1",
        )
        .bind(key)
        .bind(&job)
        .execute(&mut **tx)
        .await
        .map_err(|e| db_err("record admission", key, e))?;
        return Ok(Admission::Admitted);
    }

    // `running < limit`, so `recent > excess`: a slot frees once the `excess + 1` oldest
    // completed occupants have left the window, i.e. when the newest of them does.
    let until = sqlx::query_scalar(
        "SELECT j.started_at + make_interval(secs => $2)
         FROM concurrency_key ck
         JOIN v2_job_completed j ON j.id = ck.job_id
         WHERE ck.key = $1
           AND ck.ended_at >= now() - make_interval(secs => $2)
           AND j.started_at >= now() - make_interval(secs => $2)
         ORDER BY j.started_at
         OFFSET $3 LIMIT 1",
    )
    .bind(key)
    .bind(f64::from(window_s))
    .bind(excess)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| db_err("compute the next free slot", key, e))?;
    Ok(Admission::Deferred { until })
}

/// Puts a pulled job that `admit` deferred back in the queue, parked until `until`.
pub async fn defer(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    until: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        "UPDATE v2_job_queue
         SET running = false, started_at = NULL, worker = NULL, scheduled_for = $2,
             extras = coalesce(extras, '{}'::jsonb) || jsonb_build_object($3::text, true)
         WHERE id = $1",
    )
    .bind(job_id)
    .bind(until)
    .bind(DEFERRED_MARKER)
    .execute(&mut **tx)
    .await
    .map_err(|e| Error::internal_err(format!("Could not defer job {job_id}: {e:#}")))?;
    Ok(())
}

/// Clears the deferral marker of a job that `admit` has just admitted, so it does not travel
/// to the completed job.
pub async fn clear_deferred_marker(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
) -> Result<()> {
    sqlx::query(
        "UPDATE v2_job_queue SET extras = nullif(extras - $2, '{}'::jsonb)
         WHERE id = $1 AND extras ? $2",
    )
    .bind(job_id)
    .bind(DEFERRED_MARKER)
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        Error::internal_err(format!("Could not clear deferral of job {job_id}: {e:#}"))
    })?;
    Ok(())
}

/// Makes the longest-parked job deferred by `key`'s limit due now. Called when a job of the
/// key releases its slot; one job per release, so a backlog is not pulled all at once.
pub async fn wake_next_deferred(conn: &mut PgConnection, key: &str) -> Result<()> {
    sqlx::query(
        "UPDATE v2_job_queue SET scheduled_for = now()
         WHERE id = (
             SELECT q.id
             FROM concurrency_key ck
             JOIN v2_job_queue q ON q.id = ck.job_id
             WHERE ck.key = $1 AND ck.ended_at IS NULL
               AND q.running = false AND q.scheduled_for > now() AND q.extras ? $2
             ORDER BY q.scheduled_for, q.created_at
             LIMIT 1
             FOR UPDATE OF q SKIP LOCKED
         )",
    )
    .bind(key)
    .bind(DEFERRED_MARKER)
    .execute(conn)
    .await
    .map_err(|e| db_err("wake the next deferred job", key, e))?;
    Ok(())
}

/// Locks `key`'s counter row, creating it if the monitor removed it, and returns whether
/// `job` already holds a slot.
async fn lock_counter(
    tx: &mut Transaction<'_, Postgres>,
    key: &str,
    job: &str,
) -> Result<bool> {
    let held: Option<bool> = sqlx::query_scalar(
        "SELECT job_uuids ? $2 FROM concurrency_counter WHERE concurrency_id = $1 FOR UPDATE",
    )
    .bind(key)
    .bind(job)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| db_err("lock the counter", key, e))?;
    if let Some(held) = held {
        return Ok(held);
    }
    // The update branch takes the row lock even when a concurrent caller inserted it first.
    sqlx::query_scalar(
        "INSERT INTO concurrency_counter (concurrency_id, job_uuids) VALUES ($1, '{}'::jsonb)
         ON CONFLICT (concurrency_id) DO UPDATE SET job_uuids = concurrency_counter.job_uuids
         RETURNING job_uuids ? $2",
    )
    .bind(key)
    .bind(job)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| db_err("create the counter", key, e))
}

fn db_err(action: &str, key: &str, e: sqlx::Error) -> Error {
    Error::internal_err(format!(
        "Concurrency limit: could not {action} for key {key}: {e:#}"
    ))
}
