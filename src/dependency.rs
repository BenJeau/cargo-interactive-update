use crossterm::style::Stylize;

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

impl Dependency {
    fn versioned_name(&self) -> String {
        format!("{}@{}", self.name, self.latest_version)
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

    pub fn apply_versions(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!();

        for kind in DependencyKind::ordered() {
            self.apply_versions_by_kind(kind)?;
        }

        println!("\nDependencies have been updated!");

        Ok(())
    }

    fn apply_versions_by_kind(
        &self,
        kind: DependencyKind,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut args = self
            .0
            .iter()
            .filter(|d| d.kind == kind)
            .map(Dependency::versioned_name)
            .collect::<Vec<_>>();

        if args.is_empty() {
            return Ok(());
        }

        match kind {
            DependencyKind::Dev => args.insert(0, "--dev".to_string()),
            DependencyKind::Build => args.insert(0, "--build".to_string()),
            _ => {}
        };

        let stylized_command = format!("cargo add {}", args.join(" ").cyan()).bold();
        println!("\nExecuting {stylized_command} ...");

        std::process::Command::new("cargo")
            .arg("add")
            .args(args)
            .status()?;

        Ok(())
    }
}

impl IntoIterator for Dependencies {
    type Item = Dependency;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
