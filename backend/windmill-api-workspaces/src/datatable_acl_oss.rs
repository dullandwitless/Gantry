/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! The ACL planner: a refusal.
//!
//! Data table roles are not implemented, and neither is anything here — reading who owns what
//! included.

use {
    crate::datatable_acl::{AclChange, AclPlan, AclTarget, CatalogFacts},
    windmill_common::{datatable_roles_oss::datatable_roles_unavailable, error::Result},
};

pub(crate) fn ensure_datatable_acl_available() -> Result<()> {
    Err(datatable_roles_unavailable())
}

pub(crate) fn plan_statements(
    _target: &AclTarget,
    _change: &AclChange,
    _dbname: &str,
    _pg_role: &str,
    _facts: &CatalogFacts,
) -> Result<AclPlan> {
    Err(datatable_roles_unavailable())
}
