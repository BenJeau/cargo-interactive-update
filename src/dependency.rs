use crossterm::style::Stylize;
use std::collections::{HashMap, HashSet};
use toml_edit::{value, DocumentMut, Item, Value};

use crate::args::Args;

#[derive(Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub latest_version_date: Option<String>,
    pub current_version_date: Option<String>,
    pub kind: DependencyKind,
    pub package_name: Option<String>,
    pub workspace_path: Option<String>,
}

impl Ord for Dependency {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let ordering = self.kind.cmp(&other.kind);

        if ordering == std::cmp::Ordering::Equal {
            self.name.cmp(&other.name)
        } else {
            ordering
        }
    }
}

impl PartialOrd for Dependency {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
    Workspace,
}

impl DependencyKind {
    pub const fn ordered() -> [DependencyKind; 4] {
        [
            DependencyKind::Normal,
            DependencyKind::Dev,
            DependencyKind::Build,
            DependencyKind::Workspace,
        ]
    }
}

#[derive(Clone)]
pub struct Dependencies {
    pub dependencies: Vec<Dependency>,
    pub cargo_toml_files: HashMap<String, DocumentMut>,
}

impl Dependencies {
    pub fn new(
        dependencies: Vec<Dependency>,
        cargo_toml_files: HashMap<String, DocumentMut>,
    ) -> Self {
        Self {
            dependencies,
            cargo_toml_files,
        }
    }

    pub fn len(&self) -> usize {
        self.dependencies.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Dependency> {
        self.dependencies.iter()
    }

    pub fn apply_versions(&mut self, args: Args) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n\n");

        if self.dependencies.is_empty() {
            println!("No dependencies have been updated.");
            return Ok(());
        }

        for kind in DependencyKind::ordered() {
            self.apply_versions_by_kind(kind, args.pin);
        }

        for (workspace_path, cargo_toml) in self.cargo_toml_files.iter() {
            std::fs::write(
                format!("{}/Cargo.toml", workspace_path),
                cargo_toml.to_string(),
            )?;
            println!("Dependencies have been updated in Cargo.toml.");
        }

        if !args.no_check {
            println!("\nExecuting {}...", "cargo check".bold());
            std::process::Command::new("cargo").arg("check").status()?;
        }

        Ok(())
    }

    fn apply_versions_by_kind(&mut self, kind: DependencyKind, pin: bool) {
        for dependency in self.dependencies.iter().filter(|d| d.kind == kind) {
            let cargo_toml = self
                .cargo_toml_files
                .get_mut(
                    &dependency
                        .workspace_path
                        .clone()
                        .unwrap_or_else(|| ".".to_string()),
                )
                .unwrap();

            let version = if pin {
                value(format!("={}", dependency.latest_version))
            } else {
                value(&dependency.latest_version)
            };

            let section = match kind {
                DependencyKind::Dev => cargo_toml.get_mut("dev-dependencies"),
                DependencyKind::Build => cargo_toml.get_mut("build-dependencies"),
                DependencyKind::Workspace => cargo_toml["workspace"].get_mut("dependencies"),
                DependencyKind::Normal => cargo_toml.get_mut("dependencies"),
            }
            .unwrap();

            if matches!(section[&dependency.name], Item::Value(Value::String(_))) {
                section[&dependency.name] = version
            } else {
                section[&dependency.name]["version"] = version
            }
        }
    }

    pub fn has_workspace_members(&self) -> bool {
        self.dependencies.iter().any(|d| d.workspace_path.is_some())
    }

    pub fn filter_selected_dependencies(self, selected: Vec<bool>) -> Self {
        let mut workspace_paths = HashSet::new();
        let dependencies = self
            .dependencies
            .into_iter()
            .zip(selected.iter())
            .filter(|(_, s)| **s)
            .map(|(d, _)| {
                workspace_paths.insert(d.workspace_path.clone().unwrap_or_else(|| ".".to_string()));
                d
            })
            .collect();

        let cargo_toml_files = self
            .cargo_toml_files
            .into_iter()
            .filter(|(workspace_path, _)| workspace_paths.contains(workspace_path))
            .collect();

        Self {
            dependencies,
            cargo_toml_files,
        }
    }
}

impl IntoIterator for Dependencies {
    type Item = Dependency;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.dependencies.into_iter()
    }
}
