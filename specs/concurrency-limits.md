# Gantry Clean-Room Specification: Concurrency Limits

Author: clean-room session (Claude)
Date: 2026-11-02
Sources:
- Windmill public documentation: https://www.windmill.dev/docs/core_concepts/concurrency_limits
- `backend/windmill-api/openapi.yaml`: `concurrent_limit`, `concurrency_time_window_s`
  and `concurrency_key` on scripts, flows and flow steps; `GET /concurrency_groups/list`
- AGPL backend in this repository: `windmill-queue/src/jobs.rs` (`pull`,
  `add_completed_job`, `resolve_concurrency_key`, `insert_resolved_concurrency_key`,
  `has_active_concurrency_limit`, `try_admit_owned_job`), `windmill-queue/src/jobs_oss.rs`,
  `windmill-common/src/worker.rs` (`make_pull_query`), `src/monitor.rs` (orphaned and
  empty counter cleanup), `windmill-api-jobs/src/concurrency_groups.rs`,
  `windmill-worker/src/ai/tools.rs`, migrations creating `concurrency_counter` and
  `concurrency_key`
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. What the user sets

A script, a flow or a flow step can carry:

- **Max number of executions** (`concurrent_limit`): the cap. `0`, negative or unset
  means no limit (`has_active_concurrency_limit`).
- **Time window** (`concurrency_time_window_s`, seconds): the period the cap applies to.
  Unset or `0` means the cap counts only jobs running right now.
- **Custom concurrency key** (`concurrency_key`, optional): which jobs share the cap.
  `$workspace` and `$args[name]` are interpolated. Without one, the key is the
  runnable's path qualified by its workspace and kind (`resolve_concurrency_key`), so
  every run of the same script or flow shares it.

Keys are global: jobs from different scripts, flows or workspaces that resolve to the
same key share one cap.

## 2. What exists already

When a job with a limit is pushed, the AGPL code records its key in `concurrency_key`
and makes sure a `concurrency_counter` row exists for it. When the job completes
(`add_completed_job`, or a canceled flow in the monitor) its id is removed from
`concurrency_counter.job_uuids` and `concurrency_key.ended_at` is set. The monitor
removes ids of jobs that left the queue and deletes counters with no ids.
`GET /concurrency_groups/list` reports the number of ids per counter as the jobs running
in that group. Only the admission decision is missing: every job is admitted.

## 3. Rule

A job of key K with limit L and window W (seconds) may start when fewer than L jobs of K
**occupy** a slot. A job occupies a slot while it is admitted and still running, and,
after it completes, until W seconds after it **started**.

So with W = 0 the cap is "at most L at once"; with W > 0 it is additionally "at most L
starts in any W seconds", where a job that runs longer than W keeps its slot until it
ends.

`concurrency_counter.job_uuids` holds the ids of admitted jobs (an object keyed by job
id). Jobs counted as running are those ids whose job is still running in the queue;
ids whose job left the queue are dropped when encountered. Completed jobs are found
through `concurrency_key` (key, `ended_at` within the window) and their `started_at`.

## 4. When a worker pulls a job

After the pull query has marked a job running, if the job has an active limit, was not
canceled, and was not pulled as a resuming suspended flow:

1. Look up its key in `concurrency_key`. If there is none, the job fails with the
   existing "missing concurrency key" error.
2. In one transaction, lock the key's counter row (creating it if the monitor removed
   it). Concurrent pulls of the same key are serialized here, so the number of admitted
   jobs never exceeds L.
3. If the job is already admitted (a flow pulled again), it continues.
4. Otherwise count the occupied slots. Below L: add the job's id to `job_uuids`; the
   job runs.
5. At or above L: the job is **deferred**. It goes back to the queue, not running, with
   no worker and no start time, scheduled for the earliest moment a slot can free:
   - if fewer than L of the occupants are running, the moment enough completed
     occupants leave the window (the start of the oldest one that must leave, plus W);
   - otherwise a fixed recheck 60 seconds out, since a running job's end cannot be
     known. Completion (§5) normally wakes the job sooner.
   A line is added to the job's logs saying it was deferred by the limit of key K and
   until when. The worker then pulls again.

`DISABLE_CONCURRENCY_LIMIT=true` turns admission off, as it already does for the
counter updates on completion.

A deferred job keeps its id. It can be started early with the existing "start a deferred
job now" action, which reschedules it to now; it is then checked against the limit again
like any other pull.

## 5. When a job with a limit completes

In the same transaction that removes it from `job_uuids`, the waiting job of the same
key with the earliest `scheduled_for` (then `created_at`) that is parked in the future is
rescheduled to now. At most one job is woken per completion, so a long backlog is not
pulled all at once; a woken job that still cannot start is deferred again (§4.5).

## 6. Jobs a worker already owns

AI agent tool jobs are reserved by the worker running the agent before they start
(`try_admit_owned_job`). They are admitted by the same rule (§4 steps 2–4) without being
re-queued: if refused, the owner keeps the reservation, keeps its heartbeat and asks
again a second later.

## 7. Not covered

- Ordering between a woken job and jobs pushed at the same moment: the queue's own
  order (priority, then `scheduled_for`) decides.
- Dependency-job serialization and debouncing, which upstream ties to proprietary
  debouncing.
- The cloud-only cap on how many jobs may queue behind one key.
