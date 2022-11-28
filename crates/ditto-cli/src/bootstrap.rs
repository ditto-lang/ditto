use crate::Version;
use clap::{arg, error::ErrorKind, value_parser, ArgMatches, Command};
use console::{Emoji, Style};
use convert_case::{Case, Casing};
use ditto_config::{self as config, PackageName};
use miette::{miette, IntoDiagnostic, Result, WrapErr};
use std::{
    env::current_exe,
    fs,
    path::{Path, PathBuf},
    process,
};

pub fn command(name: impl Into<clap::builder::Str>) -> Command {
    Command::new(name)
        .about("Bootstrap a new project")
        .arg(arg!(javascript: --js "JavaScript project?"))
        .arg(arg!(--"no-make" "Don't run `ditto make`").hide(true)) // used for testing!
        .arg(arg!(--name <NAME> "Optional package name (defaults to DIR)"))
        .arg(
            arg!(directory: <DIR> "Directory for the project").value_parser(value_parser!(PathBuf)),
        )
}

#[test]
fn verify_cmd() {
    command("bootstrap").debug_assert();
}

struct Args {
    project_dir: PathBuf,
    package_name: PackageName,
    flavour: Flavour,
    no_make: bool,
}

enum Flavour {
    Bland,
    JavaScript,
}

fn matches_to_args(cmd: &mut Command, matches: &ArgMatches) -> Args {
    let project_dir = matches.get_one::<PathBuf>("directory").cloned().unwrap();

    let flavour = if matches.get_flag("javascript") {
        Flavour::JavaScript
    } else {
        Flavour::Bland
    };

    let package_name = if let Some(package_name) = matches.get_one::<String>("name").cloned() {
        if !config::PACKAGE_NAME_REGEX.is_match(&package_name) {
            cmd.error(
                ErrorKind::InvalidValue,
                format!("{:?} is not a valid package name", package_name),
            )
            .exit();
        }
        PackageName::new_unchecked(package_name)
    } else {
        let package_name = project_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        if !config::PACKAGE_NAME_REGEX.is_match(&package_name) {
            cmd.error(
                ErrorKind::InvalidValue,
                format!("{:?} cannot be used as a package name", package_name),
            )
            .exit();
        }
        PackageName::new_unchecked(package_name)
    };

    let no_make = matches.get_flag("no-make");

    Args {
        project_dir,
        package_name,
        flavour,
        no_make,
    }
}

pub fn run(cmd: &mut Command, matches: &ArgMatches, ditto_version: &Version) -> Result<()> {
    let Args {
        project_dir,
        package_name,
        flavour,
        no_make,
    } = matches_to_args(cmd, matches);

    if project_dir.exists() {
        return Err(miette!(
            "path {:?} already exists",
            project_dir.to_string_lossy()
        ));
    }

    println!("Writing files...");
    fs::create_dir_all(&project_dir)
        .into_diagnostic()
        .wrap_err(format!(
            "error creating new project directory {:?}",
            project_dir.to_string_lossy()
        ))?;

    write_files(package_name, &project_dir, ditto_version, &flavour)?;

    // Run an initial `ditto make` in the new directory to kick things off
    // unless `--no-make` is passed
    if no_make {
        return Ok(());
    }
    if let Ok(ditto) = current_exe() {
        println!("\nRunning `ditto make`...");
        process::Command::new(ditto)
            .arg("make")
            .current_dir(&project_dir)
            .status()
            .into_diagnostic()
            .wrap_err("error running `make` in new project directory")?;
    }

    Ok(())
}

fn write_files(
    package_name: PackageName,
    project_dir: &Path,
    ditto_version: &Version,
    flavour: &Flavour,
) -> Result<()> {
    let config = write_new_config(package_name, project_dir, ditto_version, flavour)?;
    write_empty_ditto_module(&config, project_dir)?;
    write_new_gitignore(&config, project_dir, flavour)?;
    match flavour {
        Flavour::Bland => {}
        Flavour::JavaScript => {
            write_js_files(&config, project_dir)?;
        }
    }
    Ok(())
}

fn write_js_files(config: &config::Config, project_dir: &Path) -> Result<()> {
    write_package_json(config, project_dir)
}

fn write_package_json(config: &config::Config, project_dir: &Path) -> Result<()> {
    let mut path = project_dir.to_path_buf();
    path.push("package");
    path.set_extension("json");
    let file = fs::File::create(&path).into_diagnostic().wrap_err(format!(
        "error creating package.json file at {:?}",
        path.to_string_lossy()
    ))?;
    let workspaces = vec![format!(
        "{}/*",
        config.codegen_js_config.packages_dir.to_string_lossy()
    )];
    let value = serde_json::json!({
        "private": true,
        "type": "module",
        "workspaces": workspaces,
    });
    serde_json::to_writer_pretty(file, &value)
        .into_diagnostic()
        .wrap_err(format!(
            "error writing package.json file to {:?}",
            path.to_string_lossy()
        ))?;
    log_path_written(path);
    Ok(())
}

fn write_new_config(
    package_name: PackageName,
    project_dir: &Path,
    ditto_version: &Version,
    flavour: &Flavour,
) -> Result<config::Config> {
    let mut config = config::Config::new(package_name);
    match flavour {
        Flavour::Bland => {}
        Flavour::JavaScript => {
            config.targets =
                //std::collections::HashSet::from([config::Target::Web, config::Target::Nodejs]);
                // TODO: uncomment the above when we can sort it for the tests
                std::collections::HashSet::from([config::Target::Nodejs]);
        }
    }
    let mut config_path = project_dir.to_path_buf();
    config_path.push(config::CONFIG_FILE_NAME);
    let config_string = toml::to_string(&config)
        .into_diagnostic()
        .wrap_err("error serializing new config file")?;

    let preamble = format!(
        "# Welcome to your new ditto project!
#
# Options for this file can be found at:
# https://github.com/ditto-lang/ditto/tree/{rev}/crates/ditto-config#readme",
        rev = ditto_version.git_rev
    );

    fs::write(&config_path, format!("{}\n{}", preamble, config_string))
        .into_diagnostic()
        .wrap_err(format!(
            "error writing new config file to {:?}",
            config_path.to_string_lossy()
        ))?;

    log_path_written(&config_path);
    Ok(config)
}

fn write_new_gitignore(
    config: &config::Config,
    project_dir: &Path,
    flavour: &Flavour,
) -> Result<()> {
    let mut path = project_dir.to_path_buf();
    path.push(".gitignore");

    let mut ignore_lines = vec![
        // .ditto
        config.ditto_dir.to_string_lossy().into_owned(),
    ];
    match flavour {
        Flavour::Bland => {}
        Flavour::JavaScript => {
            // dist
            ignore_lines.push(
                config
                    .codegen_js_config
                    .dist_dir
                    .to_string_lossy()
                    .into_owned(),
            );
            // node_modules
            ignore_lines.push(String::from("node_modules"));
        }
    }

    fs::write(&path, ignore_lines.join("\n"))
        .into_diagnostic()
        .wrap_err(format!(
            "error writing .gitignore to {:?}",
            path.to_string_lossy()
        ))?;

    log_path_written(&path);
    Ok(())
}

fn write_empty_ditto_module(config: &config::Config, project_dir: &Path) -> Result<()> {
    let mut module_path = project_dir.to_path_buf();
    module_path.push(&config.src_dir);
    fs::create_dir_all(&module_path)
        .into_diagnostic()
        .wrap_err(format!(
            "error creating ditto source directory {:?}",
            module_path.to_string_lossy()
        ))?;
    let module_name = config.name.to_case(Case::Pascal);
    module_path.push(&module_name);
    module_path.set_extension("ditto");

    let module_contents = format!("module {} exports (..);", module_name);
    write_ditto_module(module_path, module_contents)
}

fn write_ditto_module<P: AsRef<Path>>(path: P, contents: String) -> Result<()> {
    let module = ditto_cst::Module::parse(&contents).map_err(|_| {
        miette!(
            "Internal error: couldn't parse generated module: {:?}",
            contents
        )
    })?;
    let formatted = ditto_fmt::format_module(module);
    fs::write(&path, formatted)
        .into_diagnostic()
        .wrap_err(format!(
            "error writing ditto module to {}",
            path.as_ref().to_string_lossy()
        ))?;
    log_path_written(path);
    Ok(())
}

fn log_path_written<P: AsRef<Path>>(path: P) {
    if crate::common::is_plain() {
        println!("Wrote {}", path.as_ref().to_string_lossy());
    } else {
        let message = format!(
            "  {} {}",
            Emoji("✨", "Wrote"),
            path.as_ref().to_string_lossy()
        );
        println!("{}", Style::new().cyan().apply_to(message));
    }
}
