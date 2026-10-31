use {
    crate::job_logger::CompactLogs, std::io, std::sync::atomic::AtomicU32, std::sync::Arc,
    uuid::Uuid, windmill_common::DB,
};

#[allow(dead_code)]
pub(crate) async fn default_disk_log_storage(
    job_id: &Uuid,
    _w_id: &str,
    _db: &DB,
    _logs: &str,
    _total_size: Arc<AtomicU32>,
    _compact_kind: CompactLogs,
    _worker_name: &str,
) {
    tracing::info!("Logs length of {job_id} has exceeded a threshold. Implementation to store excess on disk in not OSS");
}

pub(crate) fn process_streaming_log_lines(
    r: Result<Option<String>, io::Error>,
    _stderr: bool,
    _job_id: &Uuid,
    _w_id: &str,
) -> Option<Result<String, io::Error>> {
    r.transpose()
}
