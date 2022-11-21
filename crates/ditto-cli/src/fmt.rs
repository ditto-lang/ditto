use clap::{arg, error::ErrorKind, ArgMatches, Command};
use miette::{bail, IntoDiagnostic, Result, WrapErr};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

pub fn command(name: impl Into<clap::builder::Str>) -> Command {
    Command::new(name)
        .about("Format ditto code")
        .arg(arg!(--check "Error if input(s) aren't formatted"))
        .arg(arg!(--stdin "Format stdin"))
        .arg(arg!(paths: [PATH]... "Files to format")) // TODO: support globbing
}

#[test]
fn verify_cmd() {
    command("fmt").debug_assert();
}

struct Args {
    source: Source,
    check: bool,
}

enum Source {
    Stdin,
    Paths(Vec<String>),
}

fn matches_to_args(cmd: &mut Command, matches: &ArgMatches) -> Args {
    let check = matches.get_flag("check");
    let stdin = matches.get_flag("stdin");
    let paths = matches
        .get_many::<String>("paths")
        .unwrap_or_default()
        .cloned()
        .collect::<Vec<_>>();
    if !paths.is_empty() && stdin {
        cmd.error(
            ErrorKind::ArgumentConflict,
            "Can't specify stdin and input paths",
        )
        .exit();
    }
    if stdin {
        return Args {
            source: Source::Stdin,
            check,
        };
    }
    Args {
        source: Source::Paths(paths),
        check,
    }
}

pub fn run(cmd: &mut Command, matches: &ArgMatches) -> Result<()> {
    let Args { source, check } = matches_to_args(cmd, matches);

    match source {
        Source::Stdin => {
            let mut contents = String::new();
            io::stdin()
                .read_to_string(&mut contents)
                .into_diagnostic()?;
            let formatted = fmt("stdin".into(), &contents)?;
            if check {
                if formatted != contents {
                    bail!("stdin isn't formatted");
                }
            } else {
                io::stdout()
                    .write_all(formatted.as_bytes())
                    .into_diagnostic()?;
            }
        }
        Source::Paths(paths) => {
            let mut exit_error = false;
            for path in paths {
                if check {
                    match fmt_path(&path) {
                        Err(report) => {
                            eprintln!("{:?}", report);
                            exit_error = true
                        }
                        Ok((formatted, unformatted)) => {
                            if formatted != unformatted {
                                eprintln!("{} needs formatting", path);
                                exit_error = true;
                            }
                        }
                    }
                } else {
                    eprintln!("Formatting {}", path);
                    if let Err(report) = fmt_inplace(path) {
                        eprintln!("{:?}", report);
                        exit_error = true;
                    }
                }
            }
            if exit_error {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn fmt_inplace<P: AsRef<Path>>(path: P) -> Result<()> {
    let formatted = fmt_path(&path)?.0;
    fs::write(&path, formatted)
        .into_diagnostic()
        .wrap_err(format!(
            "error writing formatted code to {}",
            path.as_ref().to_string_lossy()
        ))
}

fn fmt_path<P: AsRef<Path>>(path: P) -> Result<(String, String)> {
    // TODO gracefully handle file not existing?
    let unformatted = fs::read_to_string(&path)
        .into_diagnostic()
        .wrap_err(format!("error reading {}", path.as_ref().to_string_lossy()))?;

    let formatted = fmt(path.as_ref().to_string_lossy().into_owned(), &unformatted)?;
    Ok((formatted, unformatted))
}

pub fn fmt(name: String, contents: &str) -> Result<String> {
    // TODO `ditto-fmt` could expose a function along these lines?
    let module = ditto_cst::Module::parse(contents)
        .map_err(|err| err.into_report(&name, contents.to_string()))?;
    // TODO check that formatted file still parses if we're feeling paranoid
    Ok(ditto_fmt::format_module(module))
}
