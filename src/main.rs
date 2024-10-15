mod api;
mod cargo;
mod cli;
mod dependency;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dependencies = cargo::CargoDependencies::gather_dependencies();
    let outdated_deps = dependencies.retrieve_outdated_dependencies();

    let total_deps = dependencies.len();
    let total_outdated_deps = outdated_deps.len();

    if total_outdated_deps == 0 {
        println!("All {total_deps} direct dependencies are up to date!");
        return Ok(());
    }

    let mut state = cli::State::new(outdated_deps, total_deps);
    state.start()?;

    loop {
        state.render()?;

        match state.handle_keyboard_event()? {
            cli::Event::HandleKeyboard => {}
            cli::Event::UpdateDependencies => {
                state.selected_dependencies().apply_versions()?;
                break;
            }
            cli::Event::Exit => {
                break;
            }
        }
    }

    Ok(())
}
