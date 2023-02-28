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
    arg,
    builder::{IntoResettable, Str},
    ArgMatches, Command,
};
use miette::{IntoDiagnostic, Result};
use tracing::Instrument;
use tracing_flame::FlameLayer;
use tracing_subscriber::{prelude::*, registry::Registry};
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
        .arg(arg!(--"version-json").hide(true))
        .disable_help_subcommand(true)
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
        lsp::run(matches, version).await
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
        // Print JSON version information if called like
        // `ditto --version-json`
        if matches.get_flag("version-json") {
            println!("{}", serde_json::to_string_pretty(version).unwrap());
            return Ok(());
        }
        // Otherwise print help and exit
        cmd.print_help().unwrap();
        std::process::exit(1)
        // Or could do this...
        // clap::Error::new(clap::error::ErrorKind::MissingSubcommand).with_cmd(&cmd).exit();
    }
}

#[tokio::main]
async fn main() {
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

    if let Err(err) = try_main().await {
        eprintln!("{:?}", err);
        std::process::exit(1);
    }
    std::process::exit(0);
}

async fn try_main() -> Result<()> {
    let running_in_ninja = std::env::var("NINJA_STATUS").is_ok(); // NOTE: we set this! see make.rs

    let version = Version::from_env();
    let version_short = version.render_short();
    let version_long = version.render_long();

    let mut cmd = command(&version_short, &version_long);
    let matches = cmd.get_matches_mut();

    let mut guards = Vec::new();
    let flame_layer = if let Ok(trace_dir) = std::env::var("DITTO_TRACE_DIR") {
        let trace_dir = std::path::PathBuf::from(trace_dir);
        if !trace_dir.exists() {
            std::fs::create_dir_all(&trace_dir).into_diagnostic()?;
        }
        let args = std::env::args().collect::<Vec<_>>();
        let mut trace_file = trace_dir;
        trace_file.push(calculate_hash(&args).to_string());
        let (flame_layer, guard) = FlameLayer::with_file(trace_file).into_diagnostic()?;
        guards.push(
            // NOTE: using `Result` as a quick and easy `Either`
            Err(guard),
        );
        Some(flame_layer.with_file_and_line(false))
    } else {
        None
    };

    let fmt_layer = if let Ok(log_file) = std::env::var("DITTO_LOG_FILE") {
        let log_file = std::path::PathBuf::from(log_file);
        let log_file = if !running_in_ninja {
            if let Some(log_dir) = log_file.parent() {
                if !log_dir.exists() {
                    std::fs::create_dir_all(log_dir).into_diagnostic()?;
                }
            }
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(log_file)
                .into_diagnostic()?
        } else {
            std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(log_file)
                .into_diagnostic()?
        };
        let (non_blocking, guard) = tracing_appender::non_blocking(LogFile(log_file));
        guards.push(
            // NOTE: using `Result` as a quick and easy `Either`
            Ok(guard),
        );

        let mut fmt_layer = tracing_subscriber::fmt::Layer::new()
            .json()
            .with_writer(non_blocking);
        fmt_layer.set_ansi(false);
        Some(fmt_layer.with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE))
    } else {
        None
    };

    let subscriber = Registry::default().with(fmt_layer).with(flame_layer);
    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;

    if !running_in_ninja {
        tracing::debug!(version = version.render_short());
        run(cmd, &matches, &version).await
    } else {
        let args = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
        run(cmd, &matches, &version)
            .instrument(tracing::trace_span!("args", args = args))
            .await
    }
}

fn calculate_hash<T: std::hash::Hash>(t: &T) -> u64 {
    let mut s = std::collections::hash_map::DefaultHasher::new();
    t.hash(&mut s);
    std::hash::Hasher::finish(&s)
}

struct LogFile(std::fs::File);

impl std::io::Write for LogFile {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        // NOTE: you only pay the price for this locking if DITTO_LOG_FILE is set!
        fs2::FileExt::lock_exclusive(&self.0)?;
        let result = self.0.write(bytes);
        fs2::FileExt::unlock(&self.0)?;
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // NOTE: you only pay the price for this locking if DITTO_LOG_FILE is set!
        fs2::FileExt::lock_exclusive(&self.0)?;
        let result = self.0.flush();
        fs2::FileExt::unlock(&self.0)?;
        result
    }
}
