use rexpect::spawn;
use std::path::Path;
use toml_edit::DocumentMut;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    able_to_quit()?;

    let single_package = update_package("single_package")?;
    for (dep_type, pkg, version) in [
        ("dependencies", "base64", "0.1.0"),
        ("dev-dependencies", "unicode-ident", "1.0.0"),
        ("build-dependencies", "rand_core", "0.1.0"),
    ] {
        assert_ne!(single_package[dep_type][pkg].as_str(), Some(version));
    }

    let multiple_packages = update_package("multiple_packages")?;
    for (dep_type, pkg, version) in [
        ("dependencies", "base64", "0.1.0"),
        ("dev-dependencies", "unicode-ident", "1.0.0"),
        ("build-dependencies", "rand_core", "0.1.0"),
    ] {
        assert_ne!(multiple_packages[dep_type][pkg].as_str(), Some(version));
    }

    verify_content_of_workspaces()?;

    Ok(())
}

fn able_to_quit() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing ability to quit");

    let original_dir = std::env::current_dir()?;

    let package_path = Path::new("./packages").join("single_package");
    std::env::set_current_dir(&package_path)?;

    let mut session = spawn("cargo interactive-update", Some(3000))?;
    session.exp_string(" to select/deselect, ")?;
    session.send_line("q\r")?;
    session.exp_eof()?;

    std::env::set_current_dir(original_dir)?;
    Ok(())
}

fn update_package(package_name: &str) -> Result<DocumentMut, Box<dyn std::error::Error>> {
    println!("Updating package: {}", package_name);

    let original_dir = std::env::current_dir()?;
    let package_path = Path::new("./packages").join(package_name);
    std::env::set_current_dir(&package_path)?;

    let toml_path = Path::new("./Cargo.toml");
    let toml_content_before_update = std::fs::read_to_string(toml_path)?;

    update_all_deps()?;

    let toml_content = std::fs::read_to_string(toml_path)?;
    assert_ne!(
        toml_content, toml_content_before_update,
        "Cargo.toml was not updated"
    );

    let cargo_lock_path = Path::new("./Cargo.lock");
    assert!(cargo_lock_path.exists(), "Cargo.lock was not created");

    std::env::set_current_dir(original_dir)?;
    Ok(toml_content.parse().unwrap())
}

fn update_all_deps() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = spawn("cargo interactive-update", Some(3000))?;
    session.exp_string(" to select/deselect, ")?;
    session.send_line("a\r")?;
    session.exp_eof()?;
    Ok(())
}

fn verify_content_of_workspaces() -> Result<(), Box<dyn std::error::Error>> {
    let package_path = Path::new("./packages").join("multiple_packages");
    let first_package = Path::new(&package_path).join("first-package");
    let second_package = Path::new(&package_path).join("second-package");
    let first_package_toml = std::fs::read_to_string(Path::new(&first_package).join("Cargo.toml"))?
        .parse::<DocumentMut>()
        .unwrap();
    let second_package_toml =
        std::fs::read_to_string(Path::new(&second_package).join("Cargo.toml"))?
            .parse::<DocumentMut>()
            .unwrap();
    assert_eq!(
        first_package_toml["dependencies"]["base64"]["workspace"].as_bool(),
        Some(true)
    );
    assert_eq!(
        first_package_toml["dev-dependencies"]["unicode-ident"]["workspace"].as_bool(),
        Some(true)
    );
    assert_eq!(
        second_package_toml["build-dependencies"]["base64"]["workspace"].as_bool(),
        Some(true)
    );
    assert_ne!(
        second_package_toml["build-dependencies"]["unicode-ident"].as_str(),
        Some("1.0.0")
    );
    Ok(())
}
