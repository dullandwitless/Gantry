use axum::Router;
use uuid::Uuid;
use windmill_types::s3::StorageResourceType;

#[cfg(feature = "parquet")]
use crate::db::{ApiAuthed, OptJobAuthed, DB};
#[cfg(feature = "parquet")]
use std::sync::Arc;
#[cfg(feature = "parquet")]
use windmill_common::db::UserDB;
use windmill_common::error;
#[cfg(feature = "parquet")]
use windmill_object_store::object_store_reexports::{ObjectStore, PutMultipartOpts, PutResult};
use windmill_object_store::ObjectStoreResource;

#[cfg(feature = "parquet")]
use bytes::Bytes;
#[cfg(feature = "parquet")]
use futures::Stream;

#[cfg(feature = "parquet")]
use axum::response::Response;
#[cfg(feature = "parquet")]
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DownloadFileQuery {
    #[allow(dead_code)]
    pub file_key: String,
    #[allow(dead_code)]
    pub storage: Option<String>,
    #[allow(dead_code)]
    pub s3_resource_path: Option<String>,
}

pub fn workspaced_service() -> Router {
    Router::new()
}

#[cfg(feature = "parquet")]
pub async fn get_workspace_s3_resource<'c>(
    _authed: &ApiAuthed,
    _db: &DB,
    _user_db: Option<UserDB>,
    _w_id: &str,
    _storage: Option<String>,
) -> windmill_common::error::Result<(Option<bool>, Option<ObjectStoreResource>)> {
    // Stub: not implemented.
    Ok((None, None))
}

pub fn get_random_file_name(_file_extension: Option<String>) -> String {
    unimplemented!("Not implemented in Gantry")
}

use windmill_common::db::DbWithOptAuthed;

pub async fn get_s3_resource<'c>(
    _db_with_opt_authed: &DbWithOptAuthed<'c, ApiAuthed>,
    _w_id: &str,
    _resource_path: &str,
    _resource_type: Option<StorageResourceType>,
    _job_id: Option<Uuid>,
) -> error::Result<ObjectStoreResource> {
    Err(error::Error::internal_err(
        "Not implemented in Gantry".to_string(),
    ))
}

#[cfg(feature = "parquet")]
pub async fn upload_file_from_req(
    _s3_client: Arc<dyn ObjectStore>,
    _file_key: &str,
    _req: axum::extract::Request,
    _options: PutMultipartOpts,
    _max_size: Option<usize>,
) -> error::Result<(PutResult, usize)> {
    Err(error::Error::internal_err(
        "Not implemented in Gantry".to_string(),
    ))
}

#[cfg(feature = "parquet")]
pub async fn upload_file_internal(
    _s3_client: Arc<dyn ObjectStore>,
    _file_key: &str,
    _stream: impl Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
    _options: PutMultipartOpts,
    _max_size: Option<usize>,
) -> error::Result<(PutResult, usize)> {
    Err(error::Error::internal_err(
        "Not implemented in Gantry".to_string(),
    ))
}

// Stubs for the storage quota helpers used by apps.rs / args.rs uploads.
#[cfg(feature = "parquet")]
pub async fn ce_storage_quota_remaining(
    _db: &DB,
    _w_id: &str,
    _exclude_upload_id: Option<&str>,
) -> error::Result<i64> {
    Ok(i64::MAX)
}

#[cfg(feature = "parquet")]
pub fn reject_reserved_volume_key(_file_key: &str) -> error::Result<()> {
    Ok(())
}

#[cfg(feature = "parquet")]
pub struct CeUploadBudget {
    pub max_size: usize,
    pub existing_size: i64,
}

#[cfg(feature = "parquet")]
pub async fn ce_upload_budget(
    _db: &DB,
    _w_id: &str,
    _s3_client: &Arc<dyn ObjectStore>,
    _file_key: &str,
    _content_length: Option<i64>,
) -> error::Result<CeUploadBudget> {
    Ok(CeUploadBudget { max_size: usize::MAX, existing_size: 0 })
}

#[cfg(feature = "parquet")]
pub async fn bump_storage_usage(_db: &DB, _w_id: &str, _storage: &str, _delta: i64) {}

#[cfg(feature = "parquet")]
pub fn spawn_storage_usage_recount_floored(_db: &DB, _w_id: &str) {}

#[cfg(feature = "parquet")]
pub async fn download_s3_file_internal(
    _authed: OptJobAuthed,
    _db: &DB,
    _user_db: Option<UserDB>,
    _w_id: &str,
    _query: DownloadFileQuery,
) -> error::Result<Response> {
    Err(error::Error::internal_err(
        "Not implemented in Gantry".to_string(),
    ))
}

#[allow(dead_code)]
pub async fn read_object_streamable(
    _s3_client: Arc<dyn ObjectStore>,
    _file_key: &str,
) -> error::Result<Response> {
    Err(error::Error::internal_err(
        "Not implemented in Gantry".to_string(),
    ))
}

#[allow(dead_code)]
pub async fn delete_s3_file_internal(
    _authed: OptJobAuthed,
    _db: &DB,
    _user_db: Option<windmill_common::db::UserDB>,
    _w_id: &str,
    _query: DeleteS3FileQuery,
) -> error::Result<()> {
    Err(error::Error::internal_err(
        "Not implemented in Gantry".to_string(),
    ))
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct DeleteS3FileQuery {
    pub file_key: String,
    pub storage: Option<String>,
    pub s3_resource_path: Option<String>,
}

// Stubs for the app-scoped S3 display ops and their query/response structs.
// Only compiled with `parquet`.
#[cfg(feature = "parquet")]
mod app_s3_display_stubs {
    use super::*;
    use serde::Serialize;
    use serde_json::value::RawValue;

    #[derive(Deserialize)]
    #[allow(dead_code)]
    pub struct LoadFileMetadataQuery {
        pub file_key: String,
        pub storage: Option<String>,
        pub s3_resource_path: Option<String>,
    }

    #[derive(Serialize)]
    pub struct LoadFileMetadataResponse {}

    // Keep the required/optional fields so the build enforces the
    // query contract (e.g. the mandatory byte range) at the extraction layer.
    #[derive(Deserialize)]
    #[allow(dead_code)]
    pub struct LoadFilePreviewQuery {
        pub storage: Option<String>,
        pub s3_resource_path: Option<String>,
        pub file_key: String,
        pub file_size_in_bytes: Option<u64>,
        pub file_mime_type: Option<String>,
        pub csv_separator: Option<String>,
        pub csv_has_header: Option<bool>,
        pub read_bytes_from: u64,
        pub read_bytes_length: u64,
    }

    #[derive(Serialize)]
    pub struct LoadFilePreviewResponse {}

    #[derive(Deserialize)]
    #[allow(dead_code)]
    pub struct LoadCountQuery {
        pub search_col: Option<String>,
        pub search_term: Option<String>,
        pub storage: Option<String>,
        pub s3_resource_path: Option<String>,
        pub csv_separator: Option<String>,
        pub csv_has_header: Option<bool>,
        pub filters: Option<String>,
    }

    #[derive(Serialize)]
    pub struct TableCount {}

    #[derive(Deserialize)]
    #[allow(dead_code)]
    pub struct LoadPreviewQuery {
        pub limit: Option<u32>,
        pub offset: Option<i64>,
        pub sort_col: Option<String>,
        pub sort_desc: Option<bool>,
        pub search_col: Option<String>,
        pub search_term: Option<String>,
        pub storage: Option<String>,
        pub s3_resource_path: Option<String>,
        pub csv_separator: Option<String>,
        pub csv_has_header: Option<bool>,
        pub filters: Option<String>,
    }

    pub async fn load_file_metadata_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _user_db: Option<UserDB>,
        _w_id: &str,
        _query: LoadFileMetadataQuery,
    ) -> error::Result<LoadFileMetadataResponse> {
        Err(error::Error::internal_err(
            "Not implemented in Gantry".to_string(),
        ))
    }

    pub async fn load_file_preview_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _user_db: Option<UserDB>,
        _w_id: &str,
        _query: LoadFilePreviewQuery,
    ) -> error::Result<LoadFilePreviewResponse> {
        Err(error::Error::internal_err(
            "Not implemented in Gantry".to_string(),
        ))
    }

    pub async fn load_table_count_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _user_db: Option<UserDB>,
        _w_id: &str,
        _file_key: String,
        _query: LoadCountQuery,
    ) -> error::Result<TableCount> {
        Err(error::Error::internal_err(
            "Not implemented in Gantry".to_string(),
        ))
    }

    pub async fn load_preview_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _user_db: Option<UserDB>,
        _w_id: &str,
        _file_key: String,
        _query: LoadPreviewQuery,
        _is_parquet: bool,
    ) -> error::Result<Box<RawValue>> {
        Err(error::Error::internal_err(
            "Not implemented in Gantry".to_string(),
        ))
    }

    pub async fn download_s3_parquet_file_as_csv_internal(
        _authed: OptJobAuthed,
        _db: &DB,
        _user_db: Option<UserDB>,
        _w_id: &str,
        _query: DownloadFileQuery,
    ) -> error::Result<Response> {
        Err(error::Error::internal_err(
            "Not implemented in Gantry".to_string(),
        ))
    }
}

#[cfg(feature = "parquet")]
pub use app_s3_display_stubs::*;

pub async fn get_large_file_storage(
    _db: &DB,
    _w_id: &str,
    _storage: Option<String>,
) -> windmill_common::error::Result<Option<windmill_types::s3::LargeFileStorage>> {
    Err(windmill_common::error::Error::internal_err(
        "Not implemented in Gantry".to_string(),
    ))
}

pub async fn get_workspace_s3_resource_and_check_paths<'c>(
    _db_with_opt_authed: &DbWithOptAuthed<'c, ApiAuthed>,
    _authed_api: Option<&ApiAuthed>,
    _w_id: &str,
    _storage: Option<String>,
    _paths: &[(&str, windmill_types::s3::S3Permission)],
    _job_id: Option<uuid::Uuid>,
) -> windmill_common::error::Result<(
    Option<bool>,
    Option<windmill_types::s3::ObjectStoreResource>,
)> {
    Err(windmill_common::error::Error::internal_err(
        "Not implemented in Gantry".to_string(),
    ))
}
