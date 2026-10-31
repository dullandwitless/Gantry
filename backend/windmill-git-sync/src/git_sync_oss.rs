use windmill_common::error::Result;

use crate::{DeployedObject, DB};

pub async fn handle_deployment_metadata<'c>(
    _email: &str,
    _created_by: &str,
    _db: &DB,
    _w_id: &str,
    _obj: DeployedObject,
    _deployment_message: Option<String>,
    _skip_db_insert: bool,
    _renamed_from: Option<&str>,
) -> Result<()> {
    // Stub: git sync is not implemented.
    return Ok(());
}

pub async fn tally_deployed_object_changes(
    _w_id: &str,
    _obj: &DeployedObject,
    _db: &DB,
    _renamed_from: Option<&str>,
    _origin: Option<windmill_common::deploy_origin::DeployOrigin>,
) -> Result<()> {
    // Stub: fork change tallying is not implemented.
    return Ok(());
}

pub async fn handle_fork_branch_creation<'c>(
    _email: &str,
    _created_by: &str,
    _db: &DB,
    _w_id: &str,
    _fork_workspace_id: &str,
) -> Result<Vec<uuid::Uuid>> {
    return Ok(vec![]);
}

pub async fn handle_deployment_metadata_batch<'c>(
    _email: &str,
    _created_by: &str,
    _db: &DB,
    _w_id: &str,
    _objs: Vec<DeployedObject>,
    _deployment_message: Option<String>,
) -> Result<()> {
    return Ok(());
}
