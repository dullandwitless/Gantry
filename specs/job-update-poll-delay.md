# Gantry Clean-Room Specification: Job Update Poll Delay

Author: clean-room session (Claude)
Date: 2026-11-02
Sources:
- `backend/windmill-api/openapi.yaml`: the `poll_delay_ms` query parameter ("delay between
  polling for job updates in milliseconds") on the `run_and_stream` endpoints
  (`/w/{workspace}/jobs/run_and_stream/{f,fv,p,h}/...`)
- AGPL backend in this repository: `windmill-api/src/jobs.rs` (the job update SSE loop:
  its built-in polling schedule, the 5-second ping, `TIMEOUT_SSE_STREAM`)
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## Behaviour

A job update stream polls the job and sends what changed. Without `poll_delay_ms` it keeps
its built-in schedule: every 100 ms for the first 10 polls when `fast` is set, then every
500 ms up to poll 100, then every 3 s (every 3 s throughout without `fast`).

With `poll_delay_ms` set, every poll waits that long instead, clamped to:

- at least 100 ms, the fastest the server polls on its own, so a client cannot turn the
  stream into a busy loop on the database;
- at most 5000 ms, so the stream's keep-alive ping, which is sent between polls once 5 s
  have passed since the last one, still goes out about every 5 s.

Everything else (pings, the stream timeout, what each update contains) is unchanged.
