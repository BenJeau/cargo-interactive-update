use cargo_lock::Lockfile;
use semver::{Version, VersionReq};
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, env::current_dir};
use toml_edit::{DocumentMut, Item, Value};

use crate::{
    api,
    dependency::{Dependencies, Dependency, DependencyKind},
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CargoDependency {
    pub name: String,
    pub version: String,
    pub package: Option<String>,
    pub kind: DependencyKind,
}

impl CargoDependency {
    fn get_latest_version_wrapper(
        &self,
        workspace_member: Option<String>,
        workspace_path: Option<String>,
    ) -> Option<Dependency> {
        let parsed_current_version_req = VersionReq::parse(&self.version).ok()?;

        let response = api::get_latest_version(self)
            .expect(&format!("Unable to reach crates.io for {}", self.name))?;

        let parsed_latest_version = Version::parse(&response.latest_version).expect(&format!(
            "Latest version is not a valid semver for {}",
            self.name
        ));

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
                workspace_member,
                workspace_path,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CargoDependencies {
    pub cargo_toml: DocumentMut,
    package_name: String,
    dependencies: Vec<CargoDependency>,
    workspace_members: HashMap<String, Box<CargoDependencies>>,
}

impl CargoDependencies {
    pub fn gather_dependencies() -> Self {
        Self::gather_dependencies_inner(".", &read_cargo_lock_file(), true)
    }

    fn gather_dependencies_inner(
        relative_path: &str,
        lockfile: &Lockfile,
        should_retrieve_workspace_members: bool,
    ) -> Self {
        let cargo_toml = read_cargo_file(relative_path);
        let package_name = get_package_name(&cargo_toml);
        let dependencies = get_cargo_dependencies(&cargo_toml, lockfile);
        let workspace_members = if should_retrieve_workspace_members {
            get_workspace_members(&cargo_toml, lockfile)
        } else {
            Default::default()
        };

        Self {
            cargo_toml,
            package_name,
            dependencies,
            workspace_members,
        }
    }

    pub fn retrieve_outdated_dependencies(self, workspace_path: Option<String>) -> Dependencies {
        let mut direct_dependencies_threads = Vec::new();
        let mut workspace_member_threads = Vec::new();
        let mut cargo_toml_files = HashMap::new();
        let active_requests = Arc::new(Mutex::new(0));

        cargo_toml_files.insert(
            workspace_path.clone().unwrap_or_else(|| ".".to_string()),
            self.cargo_toml,
        );
        for dependency in self.dependencies.iter() {
            let dependency = dependency.clone();
            let package_name = self.package_name.to_string();
            let workspace_path = workspace_path.clone();
            let active_requests = active_requests.clone();

            direct_dependencies_threads.push(std::thread::spawn(move || {
                loop {
                    let mut count = active_requests.lock().unwrap();
                    if *count < 5 {
                        *count += 1;
                        break;
                    }
                    drop(count);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                let result =
                    dependency.get_latest_version_wrapper(Some(package_name), workspace_path);

                *active_requests.lock().unwrap() -= 1;

                result
            }));
        }

        for (member, dependencies) in self.workspace_members.iter() {
            let dependencies = dependencies.clone();
            let member = member.clone();
            workspace_member_threads.push(std::thread::spawn(move || {
                dependencies.retrieve_outdated_dependencies(Some(member))
            }));
        }

        let mut dependencies = direct_dependencies_threads
            .into_iter()
            .flat_map(|t| t.join())
            .flatten()
            .collect::<Vec<_>>();

        workspace_member_threads
            .into_iter()
            .for_each(|workspace_dependencies| {
                let _ = workspace_dependencies.join().map(|workspace_dependencies| {
                    dependencies.extend(workspace_dependencies.dependencies);
                    cargo_toml_files.extend(workspace_dependencies.cargo_toml_files);
                });
            });

        dependencies.sort();

        Dependencies::new(dependencies, cargo_toml_files)
    }

    pub fn len(&self) -> usize {
        self.dependencies.len()
            + self
                .workspace_members
                .values()
                .fold(0, |acc, deps| acc + deps.len())
    }
}

fn read_cargo_file(relative_path: &str) -> DocumentMut {
    let cargo_toml_content = std::fs::read_to_string(format!("{relative_path}/Cargo.toml"))
        .unwrap_or_else(|e| {
            eprintln!("Unable to read Cargo.toml file: {}", e);
            String::new()
        });

    cargo_toml_content
        .parse()
        .expect("Unable to parse Cargo.toml file as TOML")
}

fn get_cargo_dependencies(cargo_toml: &DocumentMut, lockfile: &Lockfile) -> Vec<CargoDependency> {
    let dependencies = extract_dependencies_from_sections(
        cargo_toml.get("dependencies"),
        DependencyKind::Normal,
        lockfile,
    );

    let dev_dependencies = extract_dependencies_from_sections(
        cargo_toml.get("dev-dependencies"),
        DependencyKind::Dev,
        lockfile,
    );

    let build_dependencies = extract_dependencies_from_sections(
        cargo_toml.get("build-dependencies"),
        DependencyKind::Build,
        lockfile,
    );

    let workspace_dependencies = extract_dependencies_from_sections(
        cargo_toml
            .get("workspace")
            .and_then(|w| w.get("dependencies")),
        DependencyKind::Workspace,
        lockfile,
    );

    dependencies
        .into_iter()
        .chain(dev_dependencies)
        .chain(build_dependencies)
        .chain(workspace_dependencies)
        .collect()
}

fn read_cargo_lock_file() -> Lockfile {
    let mut dir = current_dir().unwrap();

    // try recursing parents 7 times to find lockfile
    for _ in 0..7 {
        let path = dir.join("Cargo.lock");

        if let Ok(lockfile) = Lockfile::load(path) {
            return lockfile;
        }
        dir = if let Some(parent) = dir.parent() {
            parent.to_path_buf()
        } else {
            panic!("Unable to read Cargo.lock file");
        };
    }

    panic!("Unable to read Cargo.lock file");
}

fn extract_dependencies_from_sections(
    dependencies_section: Option<&Item>,
    kind: DependencyKind,
    lockfile: &Lockfile,
) -> Vec<CargoDependency> {
    let Some(dependencies_section) = dependencies_section else {
        return vec![];
    };

    let Some(package_deps) = dependencies_section.as_table_like() else {
        return vec![];
    };

    package_deps
        .iter()
        .flat_map(|(name, package_data)| {
            let (version_req, package) = match package_data {
                Item::Value(Value::String(v)) => (v.value().to_string(), None),
                Item::Value(Value::InlineTable(t)) => (
                    t.get("version")?.as_str()?.to_string(),
                    t.get("package")
                        .and_then(|e| e.as_str())
                        .map(|e| e.to_owned()),
                ),
                Item::Table(t) => (
                    t.get("version")?.as_str()?.to_string(),
                    t.get("package")
                        .and_then(|e| e.as_str())
                        .map(|e| e.to_owned()),
                ),
                _ => return None,
            };

            let version_req =
                VersionReq::parse(&version_req).expect("must be a valid version requirement");

            let package_name = package.as_deref().unwrap_or(name);

            let version = find_matching_package(lockfile, package_name, &version_req)
                .version
                .to_string();

            Some(CargoDependency {
                name: name.to_owned(),
                package,
                version,
                kind,
            })
        })
        .collect()
}

fn find_matching_package<'a>(
    lockfile: &'a Lockfile,
    package_name: &str,
    req: &VersionReq,
) -> &'a cargo_lock::Package {
    let packages = &lockfile.packages;

    // index of the package instance
    let Ok(i) = packages.binary_search_by_key(&package_name, |p| p.name.as_str()) else {
        panic!(
            "unable to find matching crate '{package_name} = \"{}\"' in Cargo.lock",
            req
        );
    };

    let package = &packages[i];
    if req.matches(&package.version) {
        return package;
    }

    // search through packages around the found index
    // to find the crate of matching version
    if i + 1 < packages.len() {
        let package_ = packages[i + 1..]
            .iter()
            .take_while(|p| p.name.as_str() == package_name)
            .find(|p| req.matches(&p.version));
        if let Some(package) = package_ {
            return package;
        }
    }

    if i > 0 {
        let package_ = packages[..i]
            .iter()
            .rev()
            .take_while(|p| p.name.as_str() == package_name)
            .find(|p| req.matches(&p.version));
        if let Some(package) = package_ {
            return package;
        }
    }

    panic!(
        "unable to find matching crate '{package_name} = \"{}\"' in Cargo.lock",
        req
    );
}

fn get_workspace_members(
    cargo_toml: &DocumentMut,
    lockfile: &Lockfile,
) -> HashMap<String, Box<CargoDependencies>> {
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

            if member == "." {
                return acc;
            }

            acc.insert(
                member.to_string(),
                Box::new(CargoDependencies::gather_dependencies_inner(
                    member, lockfile, false,
                )),
            );
            acc
        })
}

fn get_package_name(cargo_toml: &DocumentMut) -> String {
    cargo_toml
        .get("package")
        .and_then(|i| i.get("name"))
        .and_then(|i| i.as_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_cargo_dependencies_len() {
        let cargo_dependencies = CargoDependencies {
            dependencies: vec![Default::default()],
            workspace_members: HashMap::from_iter([(
                "".to_string(),
                Box::new(CargoDependencies {
                    dependencies: vec![Default::default()],
                    ..Default::default()
                }),
            )]),
            ..Default::default()
        };
        assert_eq!(cargo_dependencies.len(), 2);
    }

    #[test]
    fn test_get_cargo_dependencies() {
        const CARGO_TOML: &str = r#"
        [dependencies]
        "dependencies" = "^0.1.0"

        [dev-dependencies]
        "dev-dependencies" = "=1.0.0"

        [build-dependencies]
        "build-dependencies" = "^2.0.0"

        [workspace.dependencies]
        "workspace-dependencies" = "^3.0.0"
        "#;

        const CARGO_LOCK: &str = r#"
        version = 4

        [[package]]
        name = "build-dependencies"
        version = "2.1.0"

        [[package]]
        name = "dependencies"
        version = "0.1.2"

        [[package]]
        name = "dev-dependencies"
        version = "1.0.0"

        [[package]]
        name = "workspace-dependencies"
        version = "3.0.0"
        "#;

        let cargo_toml: DocumentMut = CARGO_TOML.parse().unwrap();
        let lockfile = Lockfile::from_str(CARGO_LOCK).unwrap();
        let dependencies = get_cargo_dependencies(&cargo_toml, &lockfile);
        assert_eq!(dependencies.len(), 4);
        assert!(dependencies.contains(&CargoDependency {
            name: "dependencies".to_string(),
            package: None,
            version: "0.1.2".to_string(),
            kind: DependencyKind::Normal
        }));
        assert!(dependencies.contains(&CargoDependency {
            name: "dev-dependencies".to_string(),
            package: None,
            version: "1.0.0".to_string(),
            kind: DependencyKind::Dev
        }));
        assert!(dependencies.contains(&CargoDependency {
            name: "build-dependencies".to_string(),
            package: None,
            version: "2.1.0".to_string(),
            kind: DependencyKind::Build
        }));
        assert!(dependencies.contains(&CargoDependency {
            name: "workspace-dependencies".to_string(),
            package: None,
            version: "3.0.0".to_string(),
            kind: DependencyKind::Workspace
        }));
    }

    #[test]
    fn test_extract_dependencies_from_sections() {
        const CARGO_TOML: &str = r#"
        [dependencies]
        "cargo-outdated" = "0.1.0"
        "other-dependency" = { version = "1.0.0" }
        "random-dependency" = { version = "2.0.0", package = "other-name" }
        "invalid-dependency" = 123

        [dependencies.serde]
        version = "1.0.0"
        "#;

        const CARGO_LOCK: &str = r#"
        version = 4

        [[package]]
        name = "cargo-outdated"
        version = "0.1.0"

        [[package]]
        name = "other-dependency"
        version = "1.0.0"

        [[package]]
        name = "other-name"
        version = "2.0.0"

        [[package]]
        name = "serde"
        version = "1.0.0"
        "#;

        let cargo_toml: DocumentMut = CARGO_TOML.parse().unwrap();
        let lockfile = Lockfile::from_str(CARGO_LOCK).unwrap();

        let dependencies = extract_dependencies_from_sections(
            cargo_toml.get("dependencies"),
            DependencyKind::Normal,
            &lockfile,
        );

        assert_eq!(dependencies.len(), 4);
        assert!(dependencies.contains(&CargoDependency {
            name: "cargo-outdated".to_string(),
            package: None,
            version: "0.1.0".to_string(),
            kind: DependencyKind::Normal
        }));
        assert!(dependencies.contains(&CargoDependency {
            name: "other-dependency".to_string(),
            package: None,
            version: "1.0.0".to_string(),
            kind: DependencyKind::Normal
        }));
        assert!(dependencies.contains(&CargoDependency {
            name: "random-dependency".to_string(),
            package: Some("other-name".to_string()),
            version: "2.0.0".to_string(),
            kind: DependencyKind::Normal
        }));
        assert!(dependencies.contains(&CargoDependency {
            name: "serde".to_string(),
            package: None,
            version: "1.0.0".to_string(),
            kind: DependencyKind::Normal
        }));
    }

    #[test]
    fn test_extract_dependencies_with_none_dependencies_section() {
        const CARGO_LOCK: &str = r#"
        version = 4
        "#;

        let lockfile = Lockfile::from_str(CARGO_LOCK).unwrap();
        let dependencies =
            extract_dependencies_from_sections(None, DependencyKind::Normal, &lockfile);
        assert_eq!(dependencies.len(), 0);
    }

    #[test]
    fn test_extract_dependencies_with_dependencies_section_not_a_table() {
        const CARGO_LOCK: &str = r#"
        version = 4
        "#;

        let lockfile = Lockfile::from_str(CARGO_LOCK).unwrap();

        let dependencies = extract_dependencies_from_sections(
            Some(&Item::Value(Value::from(false))),
            DependencyKind::Normal,
            &lockfile,
        );
        assert_eq!(dependencies.len(), 0);
    }

    #[test]
    fn test_get_workspace_members() {
        const CARGO_TOML: &str = r#"
        [workspace]
        members = ["workspace-member-1", "workspace-member-2", 0]
        "#;

        const CARGO_LOCK: &str = r#"
        version = 4
        "#;

        let lockfile = Lockfile::from_str(CARGO_LOCK).unwrap();

        let cargo_toml = CARGO_TOML.parse().unwrap();
        let workspace_members = get_workspace_members(&cargo_toml, &lockfile);
        assert_eq!(workspace_members.len(), 2);
        assert!(workspace_members.contains_key("workspace-member-1"));
        assert!(workspace_members.contains_key("workspace-member-2"));
    }

    #[test]
    fn test_get_workspace_members_with_no_workspace() {
        const CARGO_TOML: &str = r#"
        [dependencies]
        "cargo-outdated" = "0.1.0"
        "#;

        const CARGO_LOCK: &str = r#"
        version = 4

        [[package]]
        name = "cargo-outdated"
        version = "0.1.0"
        "#;

        let cargo_toml = CARGO_TOML.parse().unwrap();
        let lockfile = Lockfile::from_str(CARGO_LOCK).unwrap();
        let workspace_members = get_workspace_members(&cargo_toml, &lockfile);
        assert_eq!(workspace_members.len(), 0);
    }

    #[test]
    fn test_get_package_name_with_no_package() {
        const CARGO_TOML: &str = r#"
        [dependencies]
        "cargo-outdated" = "0.1.0"
        "#;

        let cargo_toml = CARGO_TOML.parse().unwrap();
        let package_name = get_package_name(&cargo_toml);
        assert_eq!(package_name, "");
    }

    #[test]
    fn test_get_package_name() {
        const CARGO_TOML: &str = r#"
        [package]
        name = "cargo-outdated"
        "#;

        let cargo_toml = CARGO_TOML.parse().unwrap();
        let package_name = get_package_name(&cargo_toml);
        assert_eq!(package_name, "cargo-outdated");
    }
}
