//! The routes of features Gantry leaves out because it serves one organisation in one
//! workspace: forks, deployment requests between workspaces, archiving, and public or guest
//! access to apps. Their handlers are tangled into shared upstream code, so they stay compiled
//! and this layer answers 404 in front of them.
//!
//! Clean-room implementation of `specs/single-organisation.md` §2 and §3.
//! Attestation: written without reading Windmill's frontend source or proprietary code.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Routes under `/api/w/{workspace}/`, as `<service>/<route>`; a trailing `/` matches the
/// route and everything below it.
const CUT_WORKSPACED: &[&str] = &[
    "workspaces/create_fork",
    "workspaces/fork_creation_status/",
    "workspaces/attach_dev_workspace",
    "workspaces/detach_dev_workspace",
    "workspaces/get_dev_workspace",
    "workspaces/create_workspace_fork_branch",
    "workspaces/reset_diff_tally/",
    "workspaces/compare/",
    "workspaces/seed_full_diff/",
    "workspaces/drop_forked_datatable_databases",
    "workspaces/drop_forked_ducklake_namespaces",
    "workspaces/edit_add_admins_and_developers_to_forks",
    "workspaces/archive",
    "workspaces/edit_guest_jwt_key",
    "workspaces/guest_usage",
    "workspaces/public_app_rate_limit",
    "deployment_request",
    "deployment_request/",
    "apps_u/public_app/",
    "apps_u/guest_entry/",
    "apps_u/embed_token/",
    "apps_u/public_resource/",
];

/// Routes under `/api/`.
const CUT_GLOBAL: &[&str] = &[
    "workspaces/create_fork",
    "workspaces/unarchive/",
    "users/guests",
];

fn matches(route: &str, cut: &[&str]) -> bool {
    cut.iter().any(|c| match c.strip_suffix('/') {
        Some(prefix) => route
            .strip_prefix(prefix)
            .is_some_and(|r| r.starts_with('/')),
        None => route == *c,
    })
}

/// Whether `path` is a route of a feature Gantry leaves out.
pub fn is_cut(path: &str) -> bool {
    let Some(rest) = path.strip_prefix("/api/") else {
        return false;
    };
    let rest = rest.trim_end_matches('/');
    match rest.strip_prefix("w/").and_then(|r| r.split_once('/')) {
        Some((_workspace, route)) => matches(route, CUT_WORKSPACED),
        None => matches(rest, CUT_GLOBAL),
    }
}

pub async fn refuse_cut_routes(req: Request, next: Next) -> Response {
    if is_cut(req.uri().path()) {
        return (
            StatusCode::NOT_FOUND,
            "not part of Gantry, which serves one organisation in one workspace",
        )
            .into_response();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::is_cut;

    #[test]
    fn cut_routes_are_matched_exactly() {
        assert!(is_cut("/api/w/main/workspaces/create_fork"));
        assert!(is_cut("/api/w/main/workspaces/compare/main-fork"));
        assert!(is_cut("/api/w/main/deployment_request"));
        assert!(is_cut("/api/w/main/deployment_request/3/cancel"));
        assert!(is_cut("/api/w/main/apps_u/public_app/abc"));
        assert!(is_cut("/api/workspaces/unarchive/main"));
        assert!(is_cut("/api/users/guests"));
        assert!(is_cut("/api/w/main/workspaces/archive/"));

        assert!(!is_cut("/api/w/main/workspaces/compare"));
        assert!(!is_cut("/api/w/main/workspaces/archived"));
        assert!(!is_cut("/api/w/main/apps/get/p/f/x"));
        assert!(!is_cut("/api/workspaces/create"));
        assert!(!is_cut("/api/w/main/users/guests"));
        assert!(!is_cut("/w/main/workspaces/create_fork"));
    }
}
