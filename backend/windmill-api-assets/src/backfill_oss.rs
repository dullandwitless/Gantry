//! Stub: backfilling a range of partitions is not implemented, so the request
//! is refused. Single-partition runs with an explicit `partition` arg remain
//! available.

use windmill_common::error::{Error, Result};

use crate::{PartitionsInRangeQuery, PartitionsInRangeResponse};

pub(crate) async fn partitions_in_range(
    _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _w_id: &str,
    _q: &PartitionsInRangeQuery,
) -> Result<PartitionsInRangeResponse> {
    Err(Error::BadRequest(
        "Backfilling a range of partitions is an enterprise feature".to_string(),
    ))
}
