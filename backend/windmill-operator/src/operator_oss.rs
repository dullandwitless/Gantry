pub async fn run(_db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
    anyhow::bail!("K8s operator is not available in this build")
}
