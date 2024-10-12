#[derive(Clone)]
pub struct Dependency {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub repository: Option<String>,
    pub description: Option<String>,
    pub latest_version_date: Option<String>,
    pub current_version_date: Option<String>,
}

#[derive(Clone)]
pub struct Dependencies(Vec<Dependency>);

impl Dependencies {
    pub fn new(dependencies: Vec<Dependency>) -> Self {
        Self(dependencies)
    }

    pub fn apply_versions(&self) -> Result<(), Box<dyn std::error::Error>> {
        let deps_to_update = self
            .0
            .iter()
            .map(|d| format!("{}@{}", d.name, d.latest_version))
            .collect::<Vec<_>>();

        println!("\n\nRunning `cargo add {}` ...", deps_to_update.join(" "));

        std::process::Command::new("cargo")
            .arg("add")
            .args(deps_to_update)
            .status()?;

        println!("Dependencies have been updated!");

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Dependency> {
        self.0.iter()
    }
}

impl IntoIterator for Dependencies {
    type Item = Dependency;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
