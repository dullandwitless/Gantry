/*
 * Copyright: The Gantry contributors 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! The HTTP secret store backend is not implemented in Gantry.

use crate::error::{Error, Result};

use super::HttpStoreSettings;

/// Rejects HTTP secret store settings: the backend is not implemented, and
/// accepting them would leave secrets silently served from the database.
pub fn validate_http_store_settings(_settings: &HttpStoreSettings) -> Result<()> {
    Err(Error::BadRequest(
        "The HTTP secret store backend is not supported".to_string(),
    ))
}
