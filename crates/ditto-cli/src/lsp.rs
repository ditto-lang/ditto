use clap::{ArgMatches, Command};
use miette::Result;

pub fn command<'a>(name: &str) -> Command<'a> {
    Command::new(name)
        .about("Start up the language server")
        .disable_help_subcommand(true)
}

pub fn run(_matches: &ArgMatches) -> Result<()> {
    ditto_lsp::main()
}
