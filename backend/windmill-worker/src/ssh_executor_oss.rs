pub(crate) async fn handle_ssh_bash_job(
    _ssh_path: &str,
    _mem_peak: &mut i32,
    _canceled_by: &mut Option<windmill_queue::CanceledBy>,
    _job: &windmill_queue::MiniPulledJob,
    _conn: &windmill_common::worker::Connection,
    _client: &windmill_common::client::AuthedClient,
    _content: &str,
    _job_dir: &str,
    _worker_name: &str,
    _occupancy_metrics: &mut crate::common::OccupancyMetrics,
) -> Result<Box<serde_json::value::RawValue>, windmill_common::error::Error> {
    Err(windmill_common::error::Error::ExecutionErr(
        "SSH execution (#ssh) is an enterprise feature. Use the enterprise image, or the userland \
         SSH wrapper in examples/usecase/ssh-execution-wrapper/."
            .to_string(),
    ))
}
