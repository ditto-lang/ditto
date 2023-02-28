use clap::{arg, Arg, ArgMatches, Command};
use ditto_ast as ast;
use ditto_checker as checker;
use ditto_codegen_js as js;
use ditto_config::read_config;
use ditto_cst as cst;
use miette::{miette, IntoDiagnostic, NamedSource, Report, Result};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::common;

pub static SUBCOMMAND_AST: &str = "ast";
pub static SUBCOMMAND_JS: &str = "js";
pub static SUBCOMMAND_PACKAGE_JSON: &str = "package-json";

pub static ARG_BUILD_DIR: &str = "build-dir";
pub static ARG_INPUTS: char = 'i';
pub static ARG_OUTPUTS: char = 'o';

/// The internal compile CLI.
pub fn command(name: impl Into<clap::builder::Str>) -> Command {
    return Command::new(name)
        .subcommand(
            Command::new(SUBCOMMAND_AST)
                .arg(arg!(--"build-dir" <DIR>).required(true))
                .arg(arg_inputs())
                .arg(arg_outputs()),
        )
        .subcommand(
            Command::new(SUBCOMMAND_JS)
                .arg(arg_inputs())
                .arg(arg_outputs()),
        )
        .subcommand(
            Command::new(SUBCOMMAND_PACKAGE_JSON)
                .arg(arg_input())
                .arg(arg_output()),
        );

    fn arg_input() -> Arg {
        Arg::new("input")
            .short(ARG_INPUTS)
            .num_args(1)
            .required(true)
    }
    fn arg_inputs() -> Arg {
        Arg::new("inputs")
            .short(ARG_INPUTS)
            .num_args(1..)
            .required(true)
    }
    fn arg_output() -> Arg {
        Arg::new("output")
            .short(ARG_OUTPUTS)
            .num_args(1)
            .required(true)
    }
    fn arg_outputs() -> Arg {
        Arg::new("outputs")
            .short(ARG_OUTPUTS)
            .num_args(1..)
            .required(true)
    }
}

/// Run the program given matches from [compile].
pub fn run(matches: &ArgMatches) -> Result<()> {
    if let Some(matches) = matches.subcommand_matches(SUBCOMMAND_AST) {
        let build_dir = matches.get_one::<String>("build-dir").unwrap();
        let inputs = matches.get_many("inputs").unwrap().cloned().collect();
        let outputs = matches.get_many("outputs").unwrap().cloned().collect();
        run_ast(build_dir, inputs, outputs)
    } else if let Some(matches) = matches.subcommand_matches(SUBCOMMAND_JS) {
        let inputs = matches.get_many("inputs").unwrap().cloned().collect();
        let outputs = matches.get_many("outputs").unwrap().cloned().collect();
        run_js(inputs, outputs)
    } else if let Some(matches) = matches.subcommand_matches(SUBCOMMAND_PACKAGE_JSON) {
        let input = matches.get_one::<String>("input").unwrap();
        let output = matches.get_one::<String>("output").unwrap();
        run_package_json(input, output)
    } else {
        unreachable!()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct WarningsBundle {
    pub name: String,
    pub source: String,
    // REVIEW these warnings should really be in a deterministic order!
    pub warnings: Vec<checker::WarningReport>,
}

#[tracing::instrument(level = "trace", skip_all)]
fn run_ast(build_dir: &str, inputs: Vec<String>, outputs: Vec<String>) -> Result<()> {
    let mut ditto_input = None;
    let mut everything = checker::Everything::default();

    // Need to figure out if we're compiling a "package module",
    // and if so what the name of the package is.
    //
    // This is so that imports referencing other modules in the same package
    // resolve correctly.
    let mut output_package_name = None;
    for output in outputs.iter() {
        let path = Path::new(&output);
        if let Some(common::EXTENSION_AST | common::EXTENSION_AST_EXPORTS) = full_extension(path) {
            if let Some(parent) = path.parent() {
                // Extract a package name
                if parent.to_str() != Some(build_dir) {
                    let dir = parent
                        .file_name()
                        .and_then(|file_name| file_name.to_str())
                        .unwrap();

                    output_package_name = Some(ditto_ast::PackageName(dir.to_owned()));
                }
            }
        }
    }

    for input in inputs {
        let path = Path::new(&input);
        match full_extension(path) {
            Some(common::EXTENSION_DITTO) => {
                let mut file = std::io::BufReader::new(File::open(path).into_diagnostic()?);
                let mut contents = String::new();
                file.read_to_string(&mut contents).into_diagnostic()?;
                ditto_input = Some((path.to_string_lossy().into_owned(), contents));
            }
            Some(common::EXTENSION_AST_EXPORTS) => {
                let (module_name, module_exports) = common::deserialize(path)?;

                let mut package_name = None;
                if let Some(parent) = path.parent() {
                    if parent.to_str() != Some(build_dir) {
                        let dir = parent
                            .file_name()
                            .and_then(|file_name| file_name.to_str())
                            .unwrap();

                        package_name = Some(ditto_ast::PackageName(dir.to_owned()));
                    }
                }
                if output_package_name == package_name {
                    // This is a "package module" importing another module from
                    // the same package, so it needs to be added to the typing
                    // environment _without_ a package qualifier.
                    everything.modules.insert(module_name, module_exports);
                } else if let Some(package_name) = package_name {
                    // UPSERT the package
                    if let Some(package) = everything.packages.get_mut(&package_name) {
                        package.insert(module_name, module_exports);
                    } else {
                        let mut package = HashMap::new();
                        package.insert(module_name, module_exports);
                        everything.packages.insert(package_name, package);
                    }
                } else {
                    everything.modules.insert(module_name, module_exports);
                }
            }
            other => panic!("unexpected input extension {:#?}: {}", other, input),
        }
    }

    let (ditto_input_name, ditto_input_source) = ditto_input.unwrap();

    let cst = parse_cst(&ditto_input_source, &ditto_input_name)?;
    let (ast, warnings) = check_module(everything, cst, &ditto_input_name, &ditto_input_source)?;

    let warnings = warnings
        .into_iter()
        .map(|warning| {
            tracing::warn!("{:#?}", warning);
            warning.into_report()
        })
        .collect::<Vec<_>>();

    let mut print_warnings = true;
    for output in outputs {
        let path = Path::new(&output);
        match full_extension(path) {
            Some(common::EXTENSION_AST) => {
                let file = File::create(path).into_diagnostic()?;
                let writer = std::io::BufWriter::new(file);
                common::serialize(writer, &(&ditto_input_name, &ast))?;
            }
            Some(common::EXTENSION_AST_EXPORTS) => {
                let file = File::create(path).into_diagnostic()?;
                let writer = std::io::BufWriter::new(file);
                common::serialize(writer, &(&ast.module_name, &ast.exports))?;
            }
            Some(common::EXTENSION_CHECKER_WARNINGS) => {
                let file = File::create(path).into_diagnostic()?;
                let writer = std::io::BufWriter::new(file);
                let warnings_bundle = if warnings.is_empty() {
                    None
                } else {
                    Some(WarningsBundle {
                        name: ditto_input_name.clone(),
                        source: ditto_input_source.clone(),
                        warnings: warnings.clone(),
                    })
                };
                common::serialize(writer, &warnings_bundle)?;
                print_warnings = false;
            }
            other => panic!("unexpected output extension: {:#?}", other),
        }
    }

    if print_warnings && !warnings.is_empty() {
        let source = std::sync::Arc::new(ditto_input_source);
        for warning in warnings {
            eprintln!(
                "{:#?}",
                Report::from(warning)
                    .with_source_code(NamedSource::new(&ditto_input_name, source.clone()))
            );
        }
    }

    Ok(())
}
#[tracing::instrument(level = "trace", skip(source))]
fn parse_cst(source: &str, source_name: &str) -> Result<cst::Module> {
    let module = cst::Module::parse(source).map_err(|err| {
        tracing::error!("{:#?}", err);
        err.into_report(source_name, source.to_owned())
    })?;
    Ok(module)
}

#[tracing::instrument(level = "trace", skip(everything, cst, source))]
fn check_module(
    everything: checker::Everything,
    cst: cst::Module,
    source_name: &str,
    source: &str,
) -> Result<(ast::Module, checker::Warnings)> {
    let (module, warnings) = checker::check_module(&everything, cst).map_err(|err| {
        tracing::error!("{:#?}", err);
        err.into_report(source_name, source.to_owned())
    })?;
    Ok((module, warnings))
}

#[tracing::instrument(level = "trace", skip_all)]
fn run_js(inputs: Vec<String>, outputs: Vec<String>) -> Result<()> {
    let mut ditto_input_path = None;
    let mut ast = None;
    let mut js_output_path = None;
    //let mut dts_output_path = None;

    for input in inputs {
        let path = Path::new(&input);
        match full_extension(path) {
            Some(common::EXTENSION_AST) => {
                let (deserialized_path, deserialized_ast) =
                    common::deserialize::<(String, ast::Module)>(path)?;
                ditto_input_path = Some(deserialized_path);
                ast = Some(deserialized_ast);
            }
            other => return Err(miette!("unexpected input extension: {:#?}", other)),
        }
    }

    for output in outputs {
        let path = Path::new(&output);
        match full_extension(path) {
            Some(common::EXTENSION_JS) => {
                js_output_path = Some(path.to_path_buf());
            }
            //Some(common::EXTENSION_DTS) => {
            //    dts_output_path = Some(path.to_path_buf());
            //}
            other => return Err(miette!("unexpected output extension: {:#?}", other)),
        }
    }

    // Make sure we got everything we expected
    let ditto_input_path = ditto_input_path.ok_or_else(|| miette!("AST input not specified"))?;
    let ast = ast.ok_or_else(|| miette!("AST input not specified"))?;
    let js_output_path = js_output_path.ok_or_else(|| miette!("JS output not specified"))?;
    //let dts_output_path =
    //    dts_output_path.ok_or_else(|| miette!("TypeScript declaration output not specified"))?;

    let mut foreign_module_path = PathBuf::from(ditto_input_path);
    foreign_module_path.set_extension(common::EXTENSION_JS);
    let foreign_module_path =
        pathdiff::diff_paths(foreign_module_path, js_output_path.parent().unwrap()).unwrap();

    let codegen_config = js::Config {
        // We don't want platform specific path seperators here,
        // NodeJS will handle Unix slash paths
        foreign_module_path: path_slash::PathBufExt::to_slash_lossy(&foreign_module_path)
            .into_owned(),
        module_name_to_path: Box::new(move |(package_name, module_name)| match package_name {
            Some(package_name) => {
                format!(
                    "{}/{}.{}",
                    package_name,
                    common::module_name_to_file_stem(module_name).to_string_lossy(),
                    common::EXTENSION_JS
                )
            }
            None => {
                // Assume that JS files from the same ditto project are always going to be generated
                // into a flat directory
                format!(
                    "./{}.{}",
                    common::module_name_to_file_stem(module_name).to_string_lossy(),
                    common::EXTENSION_JS
                )
            }
        }),
    };

    let js = generate_javascript(&codegen_config, ast);

    let mut js_file = std::io::BufWriter::new(File::create(&js_output_path).into_diagnostic()?);
    js_file.write_all(js.as_bytes()).into_diagnostic()?;

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
fn generate_javascript(config: &js::Config, ast: ast::Module) -> String {
    js::codegen(config, ast)
}

/// Generates a `package.json` from a `ditto.toml` input.
#[tracing::instrument(level = "trace", skip_all)]
fn run_package_json(input: &str, output: &str) -> Result<()> {
    use serde_json::{json, Map, Value};

    let config = read_config(input)?;

    // https://stackoverflow.com/a/68558580/17263155
    let value = json!({
        "name": config.name.into_string(),
        "type": "module",
        "dependencies": config
            .dependencies
            .into_iter()
            .map(|name| (name.into_string(), String::from("*")))
            .collect::<HashMap<_, _>>(),
    });

    let mut object = if let Value::Object(object) = value {
        object
    } else {
        // Look at the macro call, it's an object.
        unreachable!()
    };

    if let Some(additions) = config.codegen_js_config.package_json_additions {
        // NOTE "name" and "type" can't be overriden
        object = merge_objects(additions, object)
    }

    let file = File::create(output).into_diagnostic()?;
    let writer = std::io::BufWriter::new(file);
    return serde_json::to_writer(writer, &object).into_diagnostic();

    type Object = Map<String, Value>;
    fn merge_objects(mut lhs: Object, mut rhs: Object) -> Object {
        let mut object = Object::new();
        let keys = lhs
            .keys()
            .chain(rhs.keys())
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        for key in keys {
            match (lhs.remove(&key), rhs.remove(&key)) {
                (None, None) => {}
                (Some(lhs_value), None) => {
                    object.insert(key, lhs_value);
                }
                (None, Some(rhs_value)) => {
                    object.insert(key, rhs_value);
                }
                (Some(lhs_value), Some(rhs_value)) => {
                    object.insert(key, merge_values(lhs_value, rhs_value));
                }
            }
        }
        object
    }
    fn merge_values(lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::Array(mut lhs_values), Value::Array(rhs_values)) => {
                lhs_values.extend(rhs_values);
                Value::Array(lhs_values)
            }
            (Value::Object(lhs_values), Value::Object(rhs_values)) => {
                Value::Object(merge_objects(lhs_values, rhs_values))
            }
            (_, rhs) => rhs, // rhs takes priority
        }
    }
}

/// Returns everything after the first dot in a path.
///
/// Useful for extensions like `.d.ts` where `path.extension` would return `.ts`.
fn full_extension(path: &Path) -> Option<&str> {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .and_then(|str| str.split_once('.'))
        .map(|parts| parts.1)
}
