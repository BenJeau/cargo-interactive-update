use std::collections::HashMap;

use crossterm::style::Stylize;
use toml_edit::{value, DocumentMut, Item, Value};

use crate::args::Args;

#[derive(Clone)]
pub struct Dependency {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub latest_version_date: Option<String>,
    pub current_version_date: Option<String>,
    pub kind: DependencyKind,
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
    dependencies: Vec<Dependency>,
    workspace_members: HashMap<String, Box<Dependencies>>,
}

impl Dependencies {
    pub fn new(
        dependencies: Vec<Dependency>,
        workspace_members: HashMap<String, Box<Dependencies>>,
    ) -> Self {
        Self {
            dependencies,
            workspace_members,
        }
    }

    pub fn len(&self) -> usize {
        self.dependencies.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Dependency> {
        self.dependencies.iter()
    }

    pub fn apply_versions(
        &self,
        mut cargo_toml: DocumentMut,
        args: Args,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n\n");

        if self.dependencies.is_empty() {
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
        for dependency in self.dependencies.iter().filter(|d| d.kind == kind) {
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
}

impl IntoIterator for Dependencies {
    type Item = Dependency;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.dependencies.into_iter()
    }
}
