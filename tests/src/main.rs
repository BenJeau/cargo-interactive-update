use rexpect::spawn;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    able_to_quit()?;
    update_package("single_package")?;
    update_package("multiple_packages")?;
    Ok(())
}

fn update_all_deps() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = spawn("cargo interactive-update", Some(3000))?;
    session.exp_string(" to select/deselect, ")?;
    session.send_line("a\r")?;
    session.exp_eof()?;
    Ok(())
}

fn update_package(package_name: &str) -> Result<(), Box<dyn std::error::Error>> {
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
