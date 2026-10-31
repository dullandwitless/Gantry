use anyhow::anyhow;
pub async fn validate_license_key(
    _license_key: String,
    _db: Option<&windmill_common::DB>,
) -> anyhow::Result<(
    String,
    bool,
    Option<windmill_common::ee_oss::OfflineMetadata>,
)> {
    // Stub: not implemented.
    Err(anyhow!("License can't be validated in Windmill CE"))
}
