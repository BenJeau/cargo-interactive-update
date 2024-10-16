use crossterm::style::Stylize;
use toml_edit::{value, DocumentMut, Item, Value};

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
pub struct Dependencies(Vec<Dependency>);

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
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.0.is_empty() {
            println!("No dependencies have been updated.");
            return Ok(());
        }

        println!();

        for kind in DependencyKind::ordered() {
            self.apply_versions_by_kind(kind, &mut cargo_toml);
        }

        std::fs::write("Cargo.toml", cargo_toml.to_string())?;

        println!("Executing {} ...", "cargo check".bold());
        std::process::Command::new("cargo").arg("check").status()?;

        println!("\nDependencies have been updated.");

        Ok(())
    }

    fn apply_versions_by_kind(&self, kind: DependencyKind, cargo_toml: &mut DocumentMut) {
        for dependency in self.0.iter().filter(|d| d.kind == kind) {
            let version = value(&dependency.latest_version);

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
        self.0.into_iter()
    }
}
