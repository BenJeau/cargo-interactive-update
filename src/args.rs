use clap::Parser;

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo", styles = clap_cargo::style::CLAP_STYLING)]
pub enum CargoCli {
    InteractiveUpdate(Args),
}

#[derive(clap::Args)]
#[command(version, about, author, long_about = None)]
pub struct Args {
    /// Selects all dependencies to be updated
    #[arg(short, long)]
    pub all: bool,

    /// Execute without asking for confirmation
    #[arg(short, long)]
    pub yes: bool,

    /// Don't run `cargo check` after updating
    #[arg(short, long)]
    pub no_check: bool,
}
