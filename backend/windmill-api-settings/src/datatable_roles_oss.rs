/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where the data table role catalog endpoints come from: the enterprise implementation, or a
//! refusal. Roles are an Enterprise Edition feature; see `windmill_common::datatable_roles_oss`.

pub(crate) use ce::*;

mod ce {
    use windmill_api_auth::ApiAuthed;
    use windmill_common::{
        datatable_roles_oss::datatable_roles_unavailable as unavailable, error::Result,
    };

    pub(crate) async fn list_datatable_roles(_authed: ApiAuthed) -> Result<String> {
        Err(unavailable())
    }

    pub(crate) async fn create_datatable_role(_authed: ApiAuthed) -> Result<String> {
        Err(unavailable())
    }

    pub(crate) async fn update_datatable_role(_authed: ApiAuthed) -> Result<String> {
        Err(unavailable())
    }

    pub(crate) async fn delete_datatable_role(_authed: ApiAuthed) -> Result<String> {
        Err(unavailable())
    }
}
