use windmill_queue::MiniPulledJob;

pub fn add_root_flow_job_to_otlp(_queued_job: &MiniPulledJob, _success: bool) {}

pub fn set_job_span_parent(_span: &tracing::Span, _job: &MiniPulledJob, _rj: &uuid::Uuid) {}
