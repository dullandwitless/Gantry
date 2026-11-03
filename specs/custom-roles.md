# Gantry Clean-Room Specification: Custom Workspace and Instance Roles

Author: clean-room session (Claude)
Date: 2026-11-03
Sources:
- Windmill public documentation: https://www.windmill.dev/docs/core_concepts/roles_and_permissions
  ("Custom roles": superadmins replace the default Developer and Operator roles in instance
  settings with any list, as long as one remains; each role has a name, a label and
  capabilities, ranked by list order; roles still in use cannot be removed; workspace admins
  override what each role allows and may add workspace roles; unset capabilities of a workspace
  role default to the Operator role's; overrides are `role_overrides` in `settings.yaml`,
  instance roles the `workspace_roles` instance setting; changes reach every server within two
  minutes)
- `backend/windmill-api/openapi.yaml`: `GET /w/{workspace}/workspaces/roles` (`roles`,
  `overrides`, `custom_roles_enabled`, `my_capabilities`), `POST
  /w/{workspace}/workspaces/role_overrides`, the global settings API
- AGPL backend in this repository: `windmill-common/src/roles.rs` (role model, parsing,
  validation, `effective_roles`, `ensure_role_unused`, `validate_instance_roles_write`, seat
  sync, caches and change notifications), `windmill-api-workspaces/src/workspace_roles.rs`,
  `windmill-api-settings/src/lib.rs`, `windmill-common/src/instance_config.rs`
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. What exists already

The AGPL tree implements the whole role model: role definitions and capabilities, the
`workspace_roles` instance setting and its validation (at least one role, unique valid names,
coherent capabilities), workspace `role_overrides`, effective roles per workspace, the refusal to
remove a role still in use, seat accounting, and cache invalidation across servers. Two switches
turn the custom parts off: stored instance roles are ignored in favour of the defaults, changing
them is refused, and a workspace may only override the instance roles, not add its own.

## 2. Behaviour in Gantry

Gantry has no license tiers, so custom roles are always available:

- **Instance roles**: a superadmin may set `workspace_roles` to any valid list (setting it to
  null restores Developer and Operator). The stored list is what every workspace builds on.
  Removing a role is refused while a member, invite, auto-invite, instance group mapping, fork
  role mapping or workspace override still names it (existing check).
- **Workspace roles**: a workspace admin's `role_overrides` may name roles the instance does
  not define; they become workspace-only roles, listed after the instance roles by name, their
  unset capabilities taken from the Operator baseline (existing resolution).
- `GET /w/{workspace}/workspaces/roles` answers `custom_roles_enabled: true`.

Every other rule (validation, in-use checks, seats, caches) is the existing one.
