use windmill_types::s3::{ObjectStoreResource, StorageResourceType};

pub async fn get_s3_resource_internal<'c>(
    _resource_type: StorageResourceType,
    _s3_resource_value_raw: serde_json::Value,
    _gen_token: TokenGenerator<'c>,
    _db: &windmill_common::DB,
) -> windmill_common::error::Result<ObjectStoreResource> {
    todo!()
}

pub enum TokenGenerator<'c> {
    AsClient(&'c windmill_common::client::AuthedClient),
    AsServerInstance(),
}

impl<'c> TokenGenerator<'c> {
    pub async fn gen_token(
        &self,
        _audience: &str,
        _db: Option<&windmill_common::DB>,
    ) -> anyhow::Result<String> {
        todo!()
    }
}

#[cfg(feature = "parquet")]
pub(crate) async fn generate_s3_aws_oidc_resource<'c>(
    _clone: windmill_types::s3::S3AwsOidcResource,
    _token_generator: TokenGenerator<'c>,
    _init_private_key: Option<&sqlx::Pool<sqlx::Postgres>>,
) -> windmill_common::error::Result<ObjectStoreResource> {
    todo!()
}
