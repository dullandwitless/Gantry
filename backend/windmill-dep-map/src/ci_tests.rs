use uuid::Uuid;

use windmill_common::error;

pub async fn trigger_ci_tests_for_item(
    _db: &sqlx::Pool<sqlx::Postgres>,
    _w_id: &str,
    _item_path: &str,
    _item_kind: &str,
    _email: &str,
    _username: &str,
) -> error::Result<Vec<Uuid>> {
    Ok(vec![])
}

pub async fn trigger_all_ci_tests(
    _db: &sqlx::Pool<sqlx::Postgres>,
    _w_id: &str,
    _email: &str,
    _username: &str,
) -> error::Result<Vec<Uuid>> {
    Ok(vec![])
}
