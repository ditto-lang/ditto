use clap::{ArgMatches, Command};
use miette::Result;

pub fn command(name: impl Into<clap::builder::Str>) -> Command {
    Command::new(name)
        .about("Start up the language server")
        .disable_help_subcommand(true)
}

#[test]
fn verify_cmd() {
    command("lsp").debug_assert();
}

pub fn run(_matches: &ArgMatches) -> Result<()> {
    ditto_lsp::main()
}
