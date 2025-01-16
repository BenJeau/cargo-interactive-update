use crossterm::style::Stylize;
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
pub struct Dependencies(pub Vec<Dependency>);

impl Dependencies {
    pub fn new(dependencies: Vec<Dependency>) -> Self {
        Self(dependencies)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Dependency> {
        self.0.iter()
    }

    pub fn apply_versions(
        &self,
        mut cargo_toml: DocumentMut,
        args: Args,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n\n");

        if self.0.is_empty() {
            println!("No dependencies have been updated.");
            return Ok(());
        }

        for kind in DependencyKind::ordered() {
            self.apply_versions_by_kind(kind, &mut cargo_toml, args.pin);
        }

        std::fs::write("Cargo.toml", cargo_toml.to_string())?;
        println!("Dependencies have been updated in Cargo.toml.");

        if !args.no_check {
            println!("\nExecuting {}...", "cargo check".bold());
            std::process::Command::new("cargo").arg("check").status()?;
        }

        Ok(())
    }

    fn apply_versions_by_kind(
        &self,
        kind: DependencyKind,
        cargo_toml: &mut DocumentMut,
        pin: bool,
    ) {
        for dependency in self.0.iter().filter(|d| d.kind == kind) {
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
        self.0.iter().any(|d| d.workspace_path.is_some())
    }
}

impl IntoIterator for Dependencies {
    type Item = Dependency;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
