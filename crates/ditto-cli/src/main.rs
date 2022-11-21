mod bootstrap;
mod common;
mod fmt;
mod lsp;
mod make;
mod ninja;
mod pkg;
mod spinner;
mod version;

use clap::{
    builder::{IntoResettable, Str},
    ArgMatches, Command,
};
use miette::{IntoDiagnostic, Result};
use version::Version;

static SUBCOMMAND_BOOTSTRAP: &str = "bootstrap";
static SUBCOMMAND_MAKE: &str = "make";
static SUBCOMMAND_FMT: &str = "fmt";
static SUBCOMMAND_LSP: &str = "lsp";
static SUBCOMMAND_NINJA: &str = "ninja";

fn command(
    version_short: impl IntoResettable<Str>,
    version_long: impl IntoResettable<Str>,
) -> Command {
    Command::new("ditto")
        .bin_name("ditto")
        .version(version_short)
        .long_version(version_long)
        .disable_help_subcommand(true)
        .subcommand_required(true)
        .about("putting the fun in functional")
        .subcommand(bootstrap::command(SUBCOMMAND_BOOTSTRAP).display_order(0))
        .subcommand(make::command(SUBCOMMAND_MAKE).display_order(1))
        .subcommand(fmt::command(SUBCOMMAND_FMT).display_order(2))
        .subcommand(lsp::command(SUBCOMMAND_LSP).display_order(3))
        // internals!
        .subcommand(ninja::command(SUBCOMMAND_NINJA).hide(true))
        .subcommand(ditto_make::command_compile(make::COMPILE_SUBCOMMAND).hide(true))
}

async fn run(mut cmd: Command, matches: &ArgMatches, version: &Version) -> Result<()> {
    if let Some(matches) = matches.subcommand_matches(make::COMPILE_SUBCOMMAND) {
        ditto_make::run_compile(matches)
    } else if let Some(matches) = matches.subcommand_matches(SUBCOMMAND_MAKE) {
        make::run(matches, version).await
    } else if let Some(matches) = matches.subcommand_matches(SUBCOMMAND_LSP) {
        lsp::run(matches)
    } else if let Some(matches) = matches.subcommand_matches(SUBCOMMAND_NINJA) {
        ninja::run(matches).await
    } else if let Some(matches) = matches.subcommand_matches(SUBCOMMAND_FMT) {
        let cmd = cmd
            .get_subcommands_mut()
            .find(|cmd| cmd.get_name() == SUBCOMMAND_FMT)
            .unwrap();
        fmt::run(cmd, matches)
    } else if let Some(matches) = matches.subcommand_matches(SUBCOMMAND_BOOTSTRAP) {
        let cmd = cmd
            .get_subcommands_mut()
            .find(|cmd| cmd.get_name() == SUBCOMMAND_BOOTSTRAP)
            .unwrap();
        bootstrap::run(cmd, matches, version)
    } else {
        unreachable!()
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = try_main().await {
        eprintln!("{:?}", err);
        std::process::exit(1);
    }
    std::process::exit(0);
}

async fn try_main() -> Result<()> {
    // NOTE: this is here to catch any "internal compiler errors",
    // `unwrap`, `expect` (etc) which aren't _supposed_ to blow up
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("well that wasn't supposed to happen...\n");
        eprintln!("{}\n", panic_info);
        eprintln!("please please open an issue: https://github.com/ditto-lang/ditto/issues/new")
    }));

    miette::set_hook(Box::new(|_diagnostic| {
        // https://github.com/zkat/miette/blob/468843aa5c36ddac690dfe3a1fdaabe050a36563/src/handlers/theme.rs#L63
        Box::new(
            miette::GraphicalReportHandler::new().with_theme(if common::is_plain() {
                //miette::GraphicalTheme::ascii()
                miette::GraphicalTheme::unicode_nocolor()
            } else {
                miette::GraphicalTheme::unicode()
            }),
        )
    }))
    .expect("Error installing miette hook");

    let version = Version::from_env();
    let version_short = version.render_short();
    let version_long = version.render_long();

    let mut cmd = command(&version_short, &version_long);
    let matches = cmd.get_matches_mut();

    if let Ok(logs_dir) = std::env::var("DITTO_LOG_DIR") {
        let args = std::env::args().collect::<Vec<_>>();

        let subcommand_name = matches.subcommand_name();
        // TODO: make the log level configurable via an env var?
        flexi_logger::Logger::try_with_str("debug")
            .into_diagnostic()?
            .format_for_files(flexi_logger::default_format)
            .use_utc()
            .log_to_file(
                flexi_logger::FileSpec::default()
                    .directory(logs_dir)
                    .o_discriminant(subcommand_name.and_then(|subcmd| {
                        if subcmd == make::COMPILE_SUBCOMMAND {
                            // Need a discriminant for `_make` calls as there will
                            // be lots of them happening within less than a second
                            // (because ninja)
                            // (and the flexi_logger timestamp doesn't have millisecond precision?)
                            Some(calculate_hash(&args).to_string())
                        } else {
                            None
                        }
                    }))
                    .basename(
                        subcommand_name
                            .map_or(String::from("ditto"), |subcmd| format!("ditto_{}", subcmd)),
                    ),
            )
            .start()
            .into_diagnostic()?;

        log::debug!("{}", std::env::args().collect::<Vec<_>>().join(" "));
        log::debug!("{:?}", version);
    }

    run(cmd, &matches, &version).await
}

fn calculate_hash<T: std::hash::Hash>(t: &T) -> u64 {
    let mut s = std::collections::hash_map::DefaultHasher::new();
    t.hash(&mut s);
    std::hash::Hasher::finish(&s)
}
