use windmill_common::error;

pub const TARGET: &str = const_format::concatcp!(std::env::consts::OS, "_", std::env::consts::ARCH);

/// Two-tier cache load: check local disk first, then fall back to the shared object store.
///
/// "Shared" is the worker group's own store when its config overrides one, the instance store
/// otherwise — see [`windmill_object_store::get_cache_object_store`].
/// Returns `(hit, log_message)`.
pub async fn load_cache(bin_path: &str, _remote_path: &str, is_dir: bool) -> (bool, String) {
    if tokio::fs::metadata(&bin_path).await.is_ok() {
        (true, format!("loaded from local cache: {}\n", bin_path))
    } else {
        let _ = is_dir;
        (false, "".to_string())
    }
}

/// Whether this worker can push to the shared object store at all — the features are
/// compiled in and a store is loaded. False on builds without them, where `save_cache`
/// only ever writes to the worker's own disk.
pub async fn object_store_available() -> bool {
    {
        false
    }
}

/// Whether a binary/bundle is in the shared object store, ignoring the local cache.
///
/// The deploy-time prebuild asks this rather than [`exists_in_cache`]: a copy on the
/// building worker's own disk is exactly the state the prebuild exists to fix, so
/// answering from it would latch a failed upload into a permanent skip.
pub async fn exists_in_object_store(_remote_path: &str) -> bool {
    false
}

/// Fail a deploy-time prebuild whose artifact never reached the object store. `save_cache`
/// logs and swallows a failed upload, which is right for a run that has the binary locally
/// anyway — but for a prebuild the upload *is* the result, and a silent miss would be
/// latched by the next build's existence check.
pub async fn ensure_pushed_to_object_store(remote_path: &str) -> error::Result<()> {
    if exists_in_object_store(remote_path).await {
        return Ok(());
    }
    Err(error::Error::ExecutionErr(format!(
        "the binary was built but did not reach the object store at {remote_path}, \
         so no other worker can load it"
    )))
}

/// Check whether a binary/bundle exists in local cache or the shared object store.
pub async fn exists_in_cache(bin_path: &str, _remote_path: &str) -> bool {
    if tokio::fs::metadata(&bin_path).await.is_ok() {
        return true;
    } else {
        return false;
    }
}

/// Two-tier cache write: upload to the shared object store, then copy to local disk.
pub async fn save_cache(
    local_cache_path: &str,
    _remote_cache_path: &str,
    origin: &str,
    is_dir: bool,
) -> windmill_common::error::Result<String> {
    use std::path::PathBuf;

    let mut _cached_to_s3 = false;

    if true {
        if is_dir {
            // Populate a sibling temp dir then atomically publish it, so a
            // concurrent `load_cache`/`exists_in_cache` metadata() check never
            // observes a half-copied cache directory.
            let tmp_dir = format!("{}.tmp.{}", local_cache_path, uuid::Uuid::new_v4());
            if let Err(e) = windmill_common::worker::copy_dir_recursively(
                &PathBuf::from(origin),
                &PathBuf::from(&tmp_dir),
            )
            .and_then(|_| windmill_common::worker::atomic_publish_dir(&tmp_dir, local_cache_path))
            {
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return Err(e);
            }
        } else {
            windmill_common::worker::atomic_copy_file(origin, local_cache_path)?;
        }
        Ok(format!(
            "\nwrote cached binary: {} (uploaded to the object store: {_cached_to_s3})\n",
            local_cache_path
        ))
    } else if _cached_to_s3 {
        Ok(format!(
            "wrote cached binary to object store {}\n",
            local_cache_path
        ))
    } else {
        Ok("".to_string())
    }
}
