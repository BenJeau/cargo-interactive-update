use semver::Version;
use serde_json::Value;

use crate::{
    api,
    dependency::{Dependencies, Dependency},
};

#[derive(Clone)]
pub struct CargoDependency {
    pub name: String,
    pub version: String,
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
            })
        } else {
            None
        }
    }
}

pub struct CargoDependencies(Vec<CargoDependency>);

impl CargoDependencies {
    pub fn gather_dependencies() -> Self {
        let (cargo_toml, workspace_toml) = read_cargo_file();

        let mut cargo_toml_deps = get_cargo_toml_dependencies(Some(cargo_toml));
        let mut workspace_toml_deps = get_cargo_toml_dependencies(workspace_toml);

        cargo_toml_deps.append(&mut workspace_toml_deps);

        Self(cargo_toml_deps)
    }

    pub fn retrieve_outdated_dependencies(&self) -> Dependencies {
        let mut threads = Vec::new();

        for dependency in self.0.iter() {
            let dependency = dependency.clone();
            threads.push(std::thread::spawn(move || {
                dependency.get_latest_version_wrapper()
            }));
        }

        Dependencies::new(
            threads
                .into_iter()
                .filter_map(|t| t.join().unwrap())
                .collect(),
        )
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

fn read_cargo_file() -> (Value, Option<Value>) {
    let cargo_toml_content =
        std::fs::read_to_string("Cargo.toml").expect("Unable to read Cargo.toml file");

    let cargo_toml: Value =
        basic_toml::from_str(&cargo_toml_content).expect("Unable to parse Cargo.toml file as TOML");

    let workspace_toml = cargo_toml.get("workspace").cloned();

    (cargo_toml, workspace_toml)
}

fn get_cargo_toml_dependencies(cargo_toml: Option<Value>) -> Vec<CargoDependency> {
    let Some(cargo_toml) = cargo_toml else {
        return vec![];
    };

    let Some(package_deps) = cargo_toml.get("dependencies").and_then(|d| d.as_object()) else {
        return vec![];
    };

    package_deps
        .iter()
        .map(|(name, package_data)| CargoDependency {
            name: name.to_string(),
            version: match package_data {
                Value::String(v) => v.clone(),
                Value::Object(o) => o
                    .get("version")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| panic!("Unexpected value type")),
                _ => panic!("Unexpected value type"),
            },
        })
        .collect()
}
