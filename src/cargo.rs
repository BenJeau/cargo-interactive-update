use semver::Version;
use std::collections::HashMap;
use toml_edit::{DocumentMut, Item, Value};

use crate::{
    api,
    dependency::{Dependencies, Dependency, DependencyKind},
};

#[derive(Clone, PartialEq, Eq)]
pub struct CargoDependency {
    pub name: String,
    pub version: String,
    pub kind: DependencyKind,
}

impl Ord for CargoDependency {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let ordering = self.kind.cmp(&other.kind);

        if ordering == std::cmp::Ordering::Equal {
            self.name.cmp(&other.name)
        } else {
            ordering
        }
    }
}

impl PartialOrd for CargoDependency {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl CargoDependency {
    fn get_latest_version_wrapper(&self) -> Option<Dependency> {
        let parsed_current_version = Version::parse(&self.version).ok()?;

        let response = api::get_latest_version(self).expect("Unable to reach crates.io");

        let parsed_latest_version =
            Version::parse(&response.latest_version).expect("Latest version is not a valid semver");

        if parsed_current_version < parsed_latest_version {
            Some(Dependency {
                name: self.name.to_string(),
                current_version: self.version.to_string(),
                latest_version: response.latest_version,
                repository: response.repository,
                latest_version_date: response.latest_version_date,
                current_version_date: response.current_version_date,
                description: response.description,
                kind: self.kind,
            })
        } else {
            None
        }
    }
}

pub struct CargoDependencies {
    dependencies: Vec<CargoDependency>,
    pub cargo_toml: DocumentMut,
    workspace_members: HashMap<String, Box<CargoDependencies>>,
}

impl CargoDependencies {
    pub fn gather_dependencies(relative_path: &str) -> Self {
        let cargo_toml = read_cargo_file();
        let mut dependencies = get_cargo_dependencies(&cargo_toml);
        dependencies.sort();
        let workspace_members = get_workspace_members(&cargo_toml);
        Self {
            dependencies,
            cargo_toml,
            workspace_members,
        }
    }

    pub fn retrieve_outdated_dependencies(&self) -> Dependencies {
        let mut threads = Vec::new();

        for dependency in self.dependencies.iter() {
            let dependency = dependency.clone();
            threads.push(std::thread::spawn(move || {
                dependency.get_latest_version_wrapper()
            }));
        }

        Dependencies::new(
            threads
                .into_iter()
                .flat_map(|t| t.join())
                .flatten()
                .collect(),
        )
    }

    pub fn len(&self) -> usize {
        self.dependencies.len()
    }
}

fn read_cargo_file() -> DocumentMut {
    let cargo_toml_content =
        std::fs::read_to_string("Cargo.toml").expect("Unable to read Cargo.toml file");

    cargo_toml_content
        .parse()
        .expect("Unable to parse Cargo.toml file as TOML")
}

fn get_cargo_dependencies(cargo_toml: &DocumentMut) -> Vec<CargoDependency> {
    let dependencies =
        extract_dependencies_from_sections(cargo_toml.get("dependencies"), DependencyKind::Normal);

    let dev_dependencies =
        extract_dependencies_from_sections(cargo_toml.get("dev-dependencies"), DependencyKind::Dev);

    let build_dependencies = extract_dependencies_from_sections(
        cargo_toml.get("build-dependencies"),
        DependencyKind::Build,
    );

    let workspace_dependencies = extract_dependencies_from_sections(
        cargo_toml
            .get("workspace")
            .and_then(|w| w.get("dependencies")),
        DependencyKind::Workspace,
    );

    dependencies
        .into_iter()
        .chain(dev_dependencies)
        .chain(build_dependencies)
        .chain(workspace_dependencies)
        .collect()
}

fn extract_dependencies_from_sections(
    cargo_toml: Option<&Item>,
    kind: DependencyKind,
) -> Vec<CargoDependency> {
    let Some(cargo_toml) = cargo_toml else {
        return vec![];
    };

    let Some(package_deps) = cargo_toml.as_table_like() else {
        return vec![];
    };

    package_deps
        .iter()
        .flat_map(|(name, package_data)| {
            let version = match package_data {
                Item::Value(Value::String(v)) => v.value().to_string(),
                Item::Value(Value::InlineTable(t)) => t.get("version")?.as_str()?.to_string(),
                Item::Table(t) => t.get("version")?.as_str()?.to_string(),
                _ => return None,
            };

            Some(CargoDependency {
                name: name.to_string(),
                version,
                kind,
            })
        })
        .collect()
}

fn get_workspace_members(cargo_toml: &DocumentMut) -> HashMap<String, Box<CargoDependencies>> {
    let Some(workspace_members) = cargo_toml
        .get("workspace")
        .and_then(|i| i.get("members"))
        .and_then(|i| i.as_array())
    else {
        return HashMap::new();
    };

    workspace_members
        .iter()
        .fold(HashMap::new(), |mut acc, member| {
            let Some(member) = member.as_str() else {
                return acc;
            };

            acc.insert(
                member.to_string(),
                Box::new(CargoDependencies::gather_dependencies(member)),
            );
            acc
        })
}
