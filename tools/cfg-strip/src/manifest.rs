//! Removes disabled features from `Cargo.toml` manifests, along with optional
//! dependencies that only those features enabled.

use std::collections::BTreeSet;

use toml_edit::{DocumentMut, Item, Table, Value};

const DEP_TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

pub struct ManifestEdit {
    pub text: String,
    /// Optional dependencies dropped because nothing enables them any more.
    pub removed_deps: BTreeSet<String>,
}

/// Returns `None` if the manifest needs no change.
pub fn strip(text: &str, disabled: &BTreeSet<String>) -> anyhow::Result<Option<ManifestEdit>> {
    let mut doc: DocumentMut = text.parse()?;
    let Some(features) = doc.get_mut("features").and_then(Item::as_table_mut) else {
        return Ok(None);
    };

    let mut orphan_candidates = BTreeSet::new();
    let mut changed = false;
    for name in disabled {
        if let Some(item) = features.remove(name) {
            changed = true;
            orphan_candidates.extend(
                feature_entries(&item)
                    .filter_map(dep_name)
                    .map(String::from),
            );
        }
    }
    for (_, item) in features.iter_mut() {
        let Some(entries) = item.as_array_mut() else {
            continue;
        };
        let before = entries.len();
        entries.retain(|v| v.as_str().is_none_or(|e| !enables_disabled(e, disabled)));
        if entries.len() != before {
            changed = true;
            // In a one-line array the new first element inherits a separator space.
            if let Some(first) = entries.get_mut(0) {
                let decor = first.decor_mut();
                if decor.prefix().and_then(|p| p.as_str()) == Some(" ") {
                    decor.set_prefix("");
                }
            }
        }
    }
    if !changed {
        return Ok(None);
    }

    let still_referenced: BTreeSet<String> = features
        .iter()
        .flat_map(|(_, item)| {
            feature_entries(item)
                .filter_map(dep_name)
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .collect();
    let mut removed_deps = BTreeSet::new();
    for table in dependency_tables(&mut doc) {
        for dep in orphan_candidates.difference(&still_referenced) {
            if table.get(dep).is_some_and(is_optional) {
                table.remove(dep);
                removed_deps.insert(dep.clone());
            }
        }
    }

    Ok(Some(ManifestEdit {
        text: doc.to_string(),
        removed_deps,
    }))
}

/// Drops `[workspace.dependencies]` entries for `deps` that no member uses.
pub fn prune_workspace_deps(
    text: &str,
    deps: &BTreeSet<String>,
    used: &BTreeSet<String>,
) -> anyhow::Result<Option<String>> {
    let mut doc: DocumentMut = text.parse()?;
    let Some(table) = doc
        .get_mut("workspace")
        .and_then(|w| w.get_mut("dependencies"))
        .and_then(Item::as_table_mut)
    else {
        return Ok(None);
    };
    let before = table.len();
    for dep in deps.difference(used) {
        table.remove(dep);
    }
    Ok((table.len() != before).then(|| doc.to_string()))
}

/// Every dependency name declared by a manifest, in any dependency table.
pub fn dependency_names(text: &str) -> anyhow::Result<BTreeSet<String>> {
    let mut doc: DocumentMut = text.parse()?;
    Ok(dependency_tables(&mut doc)
        .flat_map(|t| t.iter().map(|(k, _)| k.to_string()).collect::<Vec<_>>())
        .collect())
}

fn dependency_tables(doc: &mut DocumentMut) -> impl Iterator<Item = &mut Table> {
    let (top, targets): (Vec<_>, Vec<_>) = doc
        .iter_mut()
        .partition(|(key, _)| DEP_TABLES.contains(&key.get()));
    let top = top.into_iter().filter_map(|(_, item)| item.as_table_mut());
    let per_target = targets
        .into_iter()
        .filter(|(key, _)| key.get() == "target")
        .filter_map(|(_, item)| item.as_table_mut())
        .flat_map(|targets| targets.iter_mut().filter_map(|(_, t)| t.as_table_mut()))
        .flat_map(|target| {
            target
                .iter_mut()
                .filter(|(key, _)| DEP_TABLES.contains(&key.get()))
                .filter_map(|(_, item)| item.as_table_mut())
        });
    top.chain(per_target)
}

fn is_optional(dep: &Item) -> bool {
    dep.get("optional").and_then(Item::as_bool).unwrap_or(false)
}

fn feature_entries(item: &Item) -> impl Iterator<Item = &str> {
    item.as_array()
        .into_iter()
        .flat_map(|a| a.iter().filter_map(Value::as_str))
}

/// The dependency a feature entry refers to: `dep:x`, `x/feat` or `x?/feat`.
fn dep_name(entry: &str) -> Option<&str> {
    if let Some(dep) = entry.strip_prefix("dep:") {
        return Some(dep);
    }
    entry
        .split_once('/')
        .map(|(dep, _)| dep.trim_end_matches('?'))
}

/// Whether a feature entry turns on one of the disabled features, locally
/// (`"enterprise"`) or in a dependency (`"crate/enterprise"`, `"crate?/enterprise"`).
fn enables_disabled(entry: &str, disabled: &BTreeSet<String>) -> bool {
    let feature = entry.rsplit_once('/').map_or(entry, |(_, f)| f);
    disabled.contains(feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn disabled() -> BTreeSet<String> {
        ["enterprise", "private"].map(String::from).into()
    }

    #[test]
    fn removes_features_and_orphaned_optional_deps() {
        let text = indoc! {r#"
            [package]
            name = "x"

            [features]
            default = []
            enterprise = ["dep:saml", "common/enterprise", "dep:shared"]
            private = []
            kafka = ["dep:rdkafka", "common?/enterprise", "dep:shared"]

            [dependencies]
            common = { path = "../common" }
            saml = { version = "1", optional = true }
            shared = { version = "1", optional = true }
            rdkafka = { version = "1", optional = true }
        "#};
        let edit = strip(text, &disabled()).unwrap().unwrap();
        assert_eq!(edit.removed_deps, ["saml".to_string()].into());
        assert_eq!(
            edit.text,
            indoc! {r#"
                [package]
                name = "x"

                [features]
                default = []
                kafka = ["dep:rdkafka", "dep:shared"]

                [dependencies]
                common = { path = "../common" }
                shared = { version = "1", optional = true }
                rdkafka = { version = "1", optional = true }
            "#}
        );
    }

    #[test]
    fn untouched_manifest_is_unchanged() {
        assert!(strip("[features]\na = []\n", &disabled())
            .unwrap()
            .is_none());
        assert!(strip("[package]\nname = \"x\"\n", &disabled())
            .unwrap()
            .is_none());
    }

    #[test]
    fn prunes_unused_workspace_deps_only() {
        let text = "[workspace.dependencies]\nsaml = \"1\"\nserde = \"1\"\n";
        let deps = ["saml".to_string(), "serde".to_string()].into();
        let used = ["serde".to_string()].into();
        assert_eq!(
            prune_workspace_deps(text, &deps, &used).unwrap().unwrap(),
            "[workspace.dependencies]\nserde = \"1\"\n"
        );
    }
}
