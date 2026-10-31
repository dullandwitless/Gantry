/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Data table roles: a refusal. Roles are not implemented.
//!
//! This build creates, grants and connects as no role, and a data table saved under roles — by
//! another build — is refused rather than resolved as `admin`. A data table not under roles,
//! asked for no role, resolves as it always has.

use crate::error::Error;

/// What every roles path answers. The frontend matches this exact
/// sentence (`datatableUsableRoles.ts`) to read the refusal as "not under roles": reword both.
pub fn datatable_roles_unavailable() -> Error {
    Error::BadRequest("Data table roles are a Windmill Enterprise Edition feature".to_string())
}

pub(crate) use ce::*;

mod ce {
    use super::datatable_roles_unavailable as unavailable;
    use crate::{
        datatable_roles::{DatatableRoleCatalog, DatatableRoleCluster, InstanceDatatableRole},
        db::AuthedRef,
        error::Result,
        workspaces::{
            resolve_governing_datatable, DataTableRoleTenants, DatatableAccess, GoverningDatatable,
        },
        DB,
    };

    type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

    pub(crate) async fn read_role_catalog(
        _db: &DB,
        _cluster: DatatableRoleCluster,
    ) -> Result<DatatableRoleCatalog> {
        Err(unavailable())
    }

    pub(crate) async fn read_role_catalog_tx(
        _tx: &mut Tx<'_>,
        _cluster: DatatableRoleCluster,
    ) -> Result<DatatableRoleCatalog> {
        Err(unavailable())
    }

    pub(crate) async fn role_cluster(
        _tx: &mut Tx<'_>,
        _id: &str,
    ) -> Result<Option<DatatableRoleCluster>> {
        Err(unavailable())
    }

    pub(crate) async fn insert_role_catalog_entry(
        _tx: &mut Tx<'_>,
        _id: &str,
        _cluster: DatatableRoleCluster,
        _role: &InstanceDatatableRole,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn update_role_catalog_entry(
        _tx: &mut Tx<'_>,
        _id: &str,
        _role: &InstanceDatatableRole,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn delete_role_catalog_entry(_tx: &mut Tx<'_>, _id: &str) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn registered_instance_databases(
        _db: &DB,
        _cluster: DatatableRoleCluster,
    ) -> Result<Vec<String>> {
        Err(unavailable())
    }

    /// Nothing to converge: with no roles to admit, a managed database keeps the `CONNECT` grants
    /// it was created with, as it did before roles existed.
    pub(crate) async fn converge_connect_grants(
        _db: &DB,
        _cluster: DatatableRoleCluster,
        _dbname: &str,
    ) -> Result<()> {
        Ok(())
    }

    /// As [`converge_connect_grants`].
    pub(crate) async fn converge_connect_grants_with(
        _db: &DB,
        _cluster: DatatableRoleCluster,
        _dbname: &str,
        _catalog: &DatatableRoleCatalog,
    ) -> Result<()> {
        Ok(())
    }

    pub(crate) async fn create_datatable_role(
        _db: &DB,
        _tx: &mut Tx<'_>,
        _cluster: DatatableRoleCluster,
        _name: &str,
        _password: &str,
        _take_over: bool,
    ) -> Result<bool> {
        Err(unavailable())
    }

    pub(crate) async fn set_datatable_role_login(
        _db: &DB,
        _tx: &mut Tx<'_>,
        _cluster: DatatableRoleCluster,
        _name: &str,
        _enabled: bool,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn rename_datatable_role(
        _db: &DB,
        _tx: &mut Tx<'_>,
        _cluster: DatatableRoleCluster,
        _from: &str,
        _to: &str,
        _password: &str,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn drop_datatable_role(
        _db: &DB,
        _tx: &mut Tx<'_>,
        _cluster: DatatableRoleCluster,
        _name: &str,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn ensure_instance_db_grant_options_unchecked(
        _db: &DB,
        _cluster: DatatableRoleCluster,
        _dbname: &str,
    ) -> Result<()> {
        Err(unavailable())
    }

    /// No tenant list covers anyone: there is no role to connect as.
    pub(crate) fn can_use_datatable_role(
        _tenants: &DataTableRoleTenants,
        _authed: &AuthedRef<'_>,
    ) -> bool {
        false
    }

    pub(crate) async fn can_use_datatable_role_in_governing_workspace(
        _db: &DB,
        _governing_w_id: &str,
        _w_id: &str,
        _tenants: &DataTableRoleTenants,
        _access: &DatatableAccess<'_>,
    ) -> Result<bool> {
        Err(unavailable())
    }

    /// Reached only for a data table under roles or a caller naming a role: both are refused.
    pub(crate) async fn resolve_datatable_role_connection(
        _db: &DB,
        _w_id: &str,
        _name: &str,
        _governing: &GoverningDatatable,
        _db_resource: serde_json::Value,
        _role: Option<&str>,
        _access: DatatableAccess<'_>,
    ) -> Result<serde_json::Value> {
        Err(unavailable())
    }

    /// A data table not under roles, asked for no role or for `admin`, is not a role decision and
    /// passes, as it did before roles existed. Anything else is refused.
    pub(crate) async fn ensure_can_use_datatable_role(
        db: &DB,
        w_id: &str,
        name: &str,
        role: Option<&str>,
        _access: &DatatableAccess<'_>,
        _context: &str,
    ) -> Result<()> {
        let governing = resolve_governing_datatable(db, w_id, name).await?;
        if governing.datatable.permissions.is_none()
            && role.is_none_or(|r| r == crate::datatable_roles::ADMIN_DATATABLE_ROLE)
        {
            Ok(())
        } else {
            Err(unavailable())
        }
    }

    /// A data table not under roles is the `admin` connection for anyone who reaches it, as before
    /// roles existed. One under roles is refused.
    pub(crate) async fn ensure_datatable_admin_access(
        db: &DB,
        w_id: &str,
        name: &str,
        _access: &DatatableAccess<'_>,
    ) -> Result<()> {
        let governing = resolve_governing_datatable(db, w_id, name).await?;
        if governing.datatable.permissions.is_none() {
            Ok(())
        } else {
            Err(unavailable())
        }
    }

    /// No roles, so no set login to sweep.
    pub(crate) async fn sweep_unused_role_set_logins(_db: &DB) -> Result<()> {
        Ok(())
    }

    pub(crate) async fn forget_datatable_role_everywhere(
        _tx: &mut Tx<'_>,
        _role_id: &str,
    ) -> Result<()> {
        Err(unavailable())
    }
}
