use semver::{Version, VersionReq};
use toml_edit::{DocumentMut, Item, Value};

use crate::{
    api,
    dependency::{Dependencies, Dependency, DependencyKind},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoDependency {
    pub name: String,
    pub version: String,
    pub package: Option<String>,
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
        let parsed_current_version_req = VersionReq::parse(&self.version).ok()?;

        let response = api::get_latest_version(self).expect("Unable to reach crates.io")?;

        let parsed_latest_version =
            Version::parse(&response.latest_version).expect("Latest version is not a valid semver");

        if !parsed_current_version_req.matches(&parsed_latest_version) {
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

#[derive(Debug)]
pub struct CargoDependencies {
    dependencies: Vec<CargoDependency>,
    pub cargo_toml: DocumentMut,
}

impl CargoDependencies {
    pub fn gather_dependencies() -> Self {
        let cargo_toml = read_cargo_file();
        let mut dependencies = get_cargo_dependencies(&cargo_toml);
        dependencies.sort();
        Self {
            dependencies,
            cargo_toml,
        }
    }

    pub fn into_parts(self) -> (Dependencies, DocumentMut) {
        (
            Dependencies::new(
                self.dependencies
                    .into_iter()
                    .map(|d| std::thread::spawn(move || d.get_latest_version_wrapper()))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .flat_map(|t| t.join())
                    .flatten()
                    .collect(),
            ),
            self.cargo_toml,
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
            let (version, package) = match package_data {
                Item::Value(Value::String(v)) => (v.value().to_string(), None),
                Item::Value(Value::InlineTable(t)) => (
                    t.get("version")?.as_str()?.to_string(),
                    t.get("package")
                        .map(|e| e.as_str())
                        .flatten()
                        .map(|e| e.to_owned()),
                ),
                Item::Table(t) => (
                    t.get("version")?.as_str()?.to_string(),
                    t.get("package")
                        .map(|e| e.as_str())
                        .flatten()
                        .map(|e| e.to_owned()),
                ),
                _ => return None,
            };

            Some(CargoDependency {
                name: name.to_string(),
                package,
                version,
                kind,
            })
        })
        .collect()
}
