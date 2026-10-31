use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn group_url(port: u16, endpoint: &str, name: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/groups/{endpoint}/{name}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_group_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/groups");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "test_group",
            "summary": "A test group"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create: {}", resp.text().await?);

    // create second group
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "another_group",
            "summary": "Another group"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create another: {}", resp.text().await?);

    // create duplicate -> error
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "test_group",
            "summary": "Duplicate"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // --- get ---
    let resp = authed(client().get(group_url(port, "get", "test_group")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["name"], "test_group");
    assert_eq!(body["summary"], "A test group");

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|g| g["name"] == "test_group"));

    // --- listnames ---
    let resp = authed(client().get(format!("{base}/listnames")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let names = resp.json::<Vec<String>>().await?;
    assert!(names.contains(&"test_group".to_string()));

    // --- update ---
    let resp = authed(client().post(group_url(port, "update", "test_group")))
        .json(&json!({"summary": "Updated summary"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed(client().get(group_url(port, "get", "test_group")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated summary");

    // --- adduser ---
    let resp = authed(client().post(group_url(port, "adduser", "test_group")))
        .json(&json!({"username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "adduser: {}", resp.text().await?);

    // verify membership
    let resp = authed(client().get(group_url(port, "get", "test_group")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    let members = body["members"].as_array().unwrap();
    assert!(
        members.iter().any(|m| m.as_str() == Some("test-user")),
        "expected test-user in members, got: {:?}",
        members
    );

    // --- removeuser ---
    let resp = authed(client().post(group_url(port, "removeuser", "test_group")))
        .json(&json!({"username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- is_owner ---
    let resp = authed(client().get(group_url(port, "is_owner", "test_group")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    // --- delete ---
    let resp = authed(client().delete(group_url(port, "delete", "another_group")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // verify deleted - get should 404 or the group shouldn't appear in list
    let resp = authed(client().get(format!("{base}/listnames")))
        .send()
        .await
        .unwrap();
    let names = resp.json::<Vec<String>>().await?;
    assert!(!names.contains(&"another_group".to_string()));

    // ===== Global (instance group) endpoints =====
    let global_base = format!("http://localhost:{port}/api/groups");

    // --- create instance group ---
    let resp = authed(client().post(format!("{global_base}/create")))
        .json(&json!({"name": "test_igroup", "summary": "Test instance group"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create igroup: {}", resp.text().await?);

    // --- list instance groups ---
    let resp = authed(client().get(format!("{global_base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|g| g["name"] == "test_igroup"));

    // --- list_with_workspaces ---
    let resp = authed(client().get(format!("{global_base}/list_with_workspaces")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- get instance group ---
    let resp = authed(client().get(format!("{global_base}/get/test_igroup")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["name"], "test_igroup");
    assert_eq!(body["summary"], "Test instance group");

    // --- update instance group ---
    let resp = authed(client().post(format!("{global_base}/update/test_igroup")))
        .json(&json!({"new_summary": "Updated instance group"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update igroup: {}", resp.text().await?);

    // verify update
    let resp = authed(client().get(format!("{global_base}/get/test_igroup")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated instance group");

    // --- adduser to instance group ---
    let resp = authed(client().post(format!("{global_base}/adduser/test_igroup")))
        .json(&json!({"email": "test@windmill.dev"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "adduser igroup: {}", resp.text().await?);

    // verify membership
    let resp = authed(client().get(format!("{global_base}/get/test_igroup")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    let emails = body["emails"].as_array().unwrap();
    assert!(
        emails
            .iter()
            .any(|e| e.as_str() == Some("test@windmill.dev")),
        "expected test@windmill.dev in emails, got: {:?}",
        emails
    );

    // --- removeuser from instance group ---
    let resp = authed(client().post(format!("{global_base}/removeuser/test_igroup")))
        .json(&json!({"email": "test@windmill.dev"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- export ---
    let resp = authed(client().get(format!("{global_base}/export")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "export igroups: unexpected status {}",
        resp.status()
    );

    // --- overwrite ---
    let resp = authed(client().post(format!("{global_base}/overwrite")))
        .json(&json!([]))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "overwrite igroups: unexpected status {}",
        resp.status()
    );

    // --- delete instance group ---
    let resp = authed(client().delete(format!("{global_base}/delete/test_igroup")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "delete igroup: {}", resp.text().await?);

    // verify deleted
    let resp = authed(client().get(format!("{global_base}/list")))
        .send()
        .await
        .unwrap();
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.iter().any(|g| g["name"] == "test_igroup"));

    Ok(())
}

/// Clearing an instance group's role, or deleting a group whose role was cleared out from under
/// its members, must take the instance-level role back from them.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_instance_group_role_revoked_on_clear_and_delete(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let global_base = format!("http://localhost:{port}/api/groups");
    let email = "test2@windmill.dev";

    let is_super_admin = || async {
        sqlx::query_scalar::<_, bool>("SELECT super_admin FROM password WHERE email = $1")
            .bind(email)
            .fetch_one(&db)
            .await
    };

    let resp = authed(client().post(format!("{global_base}/create")))
        .json(&json!({ "name": "admins_grp" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "create: {}", resp.text().await?);
    let resp = authed(client().post(format!("{global_base}/update/admins_grp")))
        .json(&json!({ "new_summary": "", "instance_role": "superadmin" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "set role: {}", resp.text().await?);
    let resp = authed(client().post(format!("{global_base}/adduser/admins_grp")))
        .json(&json!({ "email": email }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "adduser: {}", resp.text().await?);
    assert!(is_super_admin().await?, "group should grant superadmin");

    let resp = authed(client().post(format!("{global_base}/update/admins_grp")))
        .json(&json!({ "new_summary": "", "instance_role": null }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "update: {}", resp.text().await?);
    assert!(
        !is_super_admin().await?,
        "clearing the group's role must revoke superadmin"
    );

    // A member left elevated while the group's stored role is already NULL.
    sqlx::query(
        "UPDATE password SET super_admin = true, role_source = 'instance_group' WHERE email = $1",
    )
    .bind(email)
    .execute(&db)
    .await?;

    let resp = authed(client().delete(format!("{global_base}/delete/admins_grp")))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "delete: {}", resp.text().await?);
    assert!(
        !is_super_admin().await?,
        "deleting a role-cleared group must revoke superadmin"
    );

    Ok(())
}

/// The upgrade migration converts every member the reconciler would evict — those whose
/// granting group was deleted and those dropped from a group that still exists — and leaves
/// still-qualifying members alone. The migration has already run against the empty test
/// database by the time this executes, so the test fabricates pre-fix state and re-executes
/// the migration's statements, which are idempotent plain UPDATEs.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_preserve_orphaned_members_migration(db: Pool<Postgres>) -> anyhow::Result<()> {
    // ghost_grp pins the statement order: it is referenced by the workspace and still has a
    // membership row, but no instance_group row. Only when the reference strip runs before
    // the conversion does ghost@ read as unconverted-by-membership nowhere and get preserved;
    // converting first would spare them on the doomed reference and then strand them.
    sqlx::raw_sql(
        r#"
        INSERT INTO workspace (id, name, owner) VALUES ('mig-ws', 'mig-ws', 'admin@windmill.dev');
        INSERT INTO workspace_settings (workspace_id, auto_invite) VALUES
          ('mig-ws', '{"instance_groups": ["gone_grp", "ghost_grp", "live_grp"], "instance_groups_roles": {"gone_grp": "admin", "ghost_grp": "developer", "live_grp": "developer"}}'::jsonb);
        INSERT INTO instance_group (name) VALUES ('live_grp');
        INSERT INTO email_to_igroup (email, igroup) VALUES
          ('live@example.com', 'live_grp'),
          ('ghost@example.com', 'ghost_grp');
        INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via) VALUES
          ('mig-ws', 'orphan', 'orphan@example.com', true, false, '{"source": "instance_group", "group": "gone_grp"}'::jsonb),
          ('mig-ws', 'droppedu', 'dropped@example.com', false, false, '{"source": "instance_group", "group": "live_grp"}'::jsonb),
          ('mig-ws', 'ghostmember', 'ghost@example.com', false, false, '{"source": "instance_group", "group": "ghost_grp"}'::jsonb),
          ('mig-ws', 'livemember', 'live@example.com', false, false, '{"source": "instance_group", "group": "live_grp"}'::jsonb);
        "#,
    )
    .execute(&db)
    .await?;

    sqlx::raw_sql(include_str!(
        "../../migrations/20260813195023_preserve_orphaned_instance_group_members.up.sql"
    ))
    .execute(&db)
    .await?;

    // Deleted-group orphan and retained-group-dropped orphan both become manual members
    // with the original group recorded; the still-qualifying member is untouched.
    for (email, expected_group) in [
        ("orphan@example.com", "gone_grp"),
        ("dropped@example.com", "live_grp"),
        ("ghost@example.com", "ghost_grp"),
    ] {
        let (source, migrated_from): (Option<String>, Option<String>) = sqlx::query_as(
            "SELECT added_via->>'source', added_via->>'migrated_from_instance_group'
             FROM usr WHERE workspace_id = 'mig-ws' AND email = $1",
        )
        .bind(email)
        .fetch_one(&db)
        .await?;
        assert_eq!(
            source.as_deref(),
            Some("manual"),
            "{email} should be converted"
        );
        assert_eq!(
            migrated_from.as_deref(),
            Some(expected_group),
            "{email} marker"
        );
    }

    let (source, group): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT added_via->>'source', added_via->>'group'
         FROM usr WHERE workspace_id = 'mig-ws' AND email = 'live@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        source.as_deref(),
        Some("instance_group"),
        "still-qualifying member spared"
    );
    assert_eq!(group.as_deref(), Some("live_grp"));

    // The dangling references are stripped from both auto_invite fields; the live one stays.
    let (groups, roles): (serde_json::Value, serde_json::Value) = sqlx::query_as(
        "SELECT auto_invite->'instance_groups', auto_invite->'instance_groups_roles'
         FROM workspace_settings WHERE workspace_id = 'mig-ws'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(groups, json!(["live_grp"]));
    assert_eq!(roles, json!({ "live_grp": "developer" }));

    Ok(())
}

/// A non-admin who creates a reserved group first becomes its member and inherits every grant
/// Windmill later gives it.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_reserved_group_names_admin_only(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace/groups",
        server.addr.port()
    );

    for name in windmill_common::RESERVED_GROUP_NAMES {
        let resp = client()
            .post(format!("{base}/create"))
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .json(&json!({ "name": name }))
            .send()
            .await?;
        assert_eq!(resp.status(), 400, "non-admin creating {name}");
        assert!(resp.text().await?.contains("reserved"));

        let resp = authed(client().post(format!("{base}/create")))
            .json(&json!({ "name": name }))
            .send()
            .await?;
        assert_eq!(
            resp.status(),
            200,
            "admin creating {name}: {}",
            resp.text().await?
        );
    }
    Ok(())
}
