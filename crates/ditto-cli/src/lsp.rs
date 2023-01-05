use crate::Version;
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

pub async fn run(_matches: &ArgMatches, ditto_version: &Version) -> Result<()> {
    ditto_lsp::main(ditto_version.render_short()).await;
    Ok(())
}
