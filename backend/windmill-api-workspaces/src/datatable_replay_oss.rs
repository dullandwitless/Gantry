/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where the replay of a data table's owners and grants into its copy comes from: the enterprise
//! one, or a refusal. Without it a data table under roles is not copied at all: its rows would
//! arrive owned by the admin connection with no grant for any role.

use {
    std::collections::BTreeSet,
    tokio::sync::mpsc,
    tokio_postgres::error::DbError,
    windmill_common::error::{Error, Result},
};

pub(crate) fn ensure_replay() -> Result<()> {
    Err(Error::BadRequest(
        "Cloning a data table under roles is not supported in Gantry: the copy \
         needs the source's owners and grants replayed. Fork it keeping the original database \
         instead."
            .to_string(),
    ))
}

pub(crate) async fn replay_owners_and_grants(
    _source: &tokio_postgres::Client,
    _target: &tokio_postgres::Transaction<'_>,
    _notices: &mut mpsc::UnboundedReceiver<DbError>,
    _catalog_roles: &BTreeSet<String>,
) -> Result<()> {
    ensure_replay()
}
