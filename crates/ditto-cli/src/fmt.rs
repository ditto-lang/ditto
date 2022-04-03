use clap::{Arg, ArgMatches, Command};
use miette::{bail, IntoDiagnostic, Result, WrapErr};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

pub fn command<'a>(name: &str) -> Command<'a> {
    Command::new(name)
        .about("Format ditto code")
        .arg(Arg::new("stdin").long("stdin"))
        .arg(Arg::new("check").long("check"))
        .arg(Arg::new("globs").takes_value(true).multiple_values(true))
}

pub fn run(matches: &ArgMatches) -> Result<()> {
    if matches.is_present("stdin") {
        if matches.is_present("globs") {
            bail!("can only specify `--stdin` or paths, not both")
        }
        let mut contents = String::new();
        io::stdin()
            .read_to_string(&mut contents)
            .into_diagnostic()?;
        let formatted = fmt("stdin".into(), &contents)?;
        if matches.is_present("check") {
            if formatted != contents {
                bail!("Stdin isn't formatted");
            }
        } else {
            io::stdout()
                .write_all(formatted.as_bytes())
                .into_diagnostic()?;
        }
    } else if let Some(globs) = matches.values_of("globs") {
        // TODO actually glob the input(s)
        let check = matches.is_present("check");
        let exit_error = false;
        for path in globs {
            if check {
                match fmt_path(path) {
                    Err(report) => {
                        eprintln!("{:?}", report);
                    }
                    Ok((formatted, unformatted)) => {
                        if formatted != unformatted {
                            eprintln!("{} needs formatting", path);
                        }
                    }
                }
            } else {
                eprintln!("Formatting {}", path);
                if let Err(report) = fmt_inplace(path) {
                    eprintln!("{:?}", report);
                }
            }
        }
        if exit_error {
            bail!("Some files need formatting");
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
