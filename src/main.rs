use clap::Parser;

mod api;
mod args;
mod cargo;
mod cli;
mod dependency;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args::CargoCli::InteractiveUpdate(args) = args::CargoCli::parse();

    let dependencies = cargo::CargoDependencies::gather_dependencies("./");
    let outdated_deps = dependencies.retrieve_outdated_dependencies(None);

    let total_deps = dependencies.len();
    let total_outdated_deps = outdated_deps.len();

    if total_outdated_deps == 0 {
        println!("All {total_deps} direct dependencies are up to date!");
        return Ok(());
    }

    println!("{total_outdated_deps} out of the {total_deps} direct dependencies are outdated.");

    let mut state = cli::State::new(outdated_deps, total_deps, args.all);

    if args.yes {
        state
            .selected_dependencies()
            .apply_versions(dependencies.cargo_toml, args)?;
        return Ok(());
    }

    state.start()?;

    loop {
        state.render()?;

        match state.handle_keyboard_event()? {
            cli::Event::HandleKeyboard => {}
            cli::Event::UpdateDependencies => {
                state
                    .selected_dependencies()
                    .apply_versions(dependencies.cargo_toml, args)?;
                break;
            }
            cli::Event::Exit => {
                break;
            }
        }
    }

    Ok(())
}
