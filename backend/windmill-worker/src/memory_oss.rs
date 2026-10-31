use {
    crate::memory_common, uuid::Uuid, windmill_ai::types::OpenAIMessage, windmill_common::db::DB,
};

pub async fn read_from_memory(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
) -> anyhow::Result<Option<Vec<OpenAIMessage>>> {
    memory_common::read_from_db(db, workspace_id, conversation_id, step_id)
        .await
        .map_err(|e| anyhow::anyhow!("Database read failed: {e:?}"))
}

pub async fn write_to_memory(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
    step_id: &str,
    messages: &[OpenAIMessage],
    allow_truncation: bool,
) -> anyhow::Result<usize> {
    if messages.is_empty() {
        return Ok(0);
    }

    memory_common::write_to_db(
        db,
        workspace_id,
        conversation_id,
        step_id,
        messages,
        allow_truncation,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Database write failed: {e:?}"))
}

pub async fn memory_storage_capacity_bytes() -> Option<usize> {
    Some(memory_common::MAX_MEMORY_SIZE_BYTES)
}

pub async fn delete_conversation_memory(
    db: &DB,
    workspace_id: &str,
    conversation_id: Uuid,
) -> anyhow::Result<()> {
    memory_common::delete_conversation_from_db(db, workspace_id, conversation_id)
        .await
        .map_err(|e| anyhow::anyhow!("Database delete failed: {e:?}"))
}
