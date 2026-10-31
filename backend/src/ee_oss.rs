pub async fn set_license_key(_license_key: String, _db: Option<&windmill_common::db::DB>) -> () {
    // Stub: not implemented.
}

#[cfg(feature = "parquet")]
pub async fn export_audit_logs_to_object_store(_db: &windmill_common::db::DB) {
    // Stub: not implemented.
}

#[cfg(feature = "parquet")]
pub async fn anchor_audit_logs_s3_checkpoint_env_var(_db: &windmill_common::db::DB) {
    // Stub: not implemented.
}
