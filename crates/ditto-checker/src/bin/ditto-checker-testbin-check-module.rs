fn parse_and_check_module(
    source: String,
    source_name: String,
    everything: &ditto_checker::Everything,
) -> ditto_ast::Module {
    match ditto_cst::Module::parse(&source) {
        Err(err) => {
            eprintln!(
                "{:?}",
                miette::Report::from(err.into_report(source_name, source))
            );
            std::process::exit(1)
        }
        Ok(cst_module) => match ditto_checker::check_module(everything, cst_module) {
            Err(err) => {
                eprintln!(
                    "{:?}",
                    miette::Report::from(err.into_report(source_name, source))
                );
                std::process::exit(1)
            }
            Ok((module, warnings)) => {
                for warning in warnings {
                    eprintln!(
                        "{:?}",
                        miette::Report::from(warning.into_report()).with_source_code(
                            miette::NamedSource::new(source_name.clone(), source.clone())
                        )
                    )
                }
                module
            }
        },
    }
}

fn read_module_dir<P: AsRef<std::path::Path>>(
    path: P,
    mut everything: ditto_checker::Everything,
) -> Vec<ditto_ast::Module> {
    let mut module_paths = Vec::new();
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        module_paths.push(path)
    }
    module_paths.sort(); // NOTE file order implies dependency order!
    let mut modules = Vec::new();
    for mut path in module_paths {
        let source = std::fs::read_to_string(&path).unwrap();
        let source_name = path.to_string_lossy().into_owned();
        let module = parse_and_check_module(source, source_name, &everything);

        path.set_extension("ast");
        let ast_file = std::fs::File::create(&path).unwrap();
        serde_json::to_writer_pretty(ast_file, &module).unwrap();

        everything
            .modules
            .insert(module.module_name.clone(), module.exports.clone());
        modules.push(module);
    }
    modules
}

fn main() {
    let mut everything = ditto_checker::Everything::default();

    let mut package_paths = Vec::new();
    for entry in std::fs::read_dir("./").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            package_paths.push(path)
        }
    }
    package_paths.sort(); // NOTE directory order implies dependency order!
    for path in package_paths {
        let modules = read_module_dir(&path, everything.clone())
            .into_iter()
            .map(|module| (module.module_name, module.exports))
            .collect::<std::collections::HashMap<_, _>>();
        everything.packages.insert(
            ditto_ast::PackageName(path.file_name().unwrap().to_string_lossy().into_owned()),
            modules,
        );
    }

    everything.modules = read_module_dir("./", everything.clone())
        .into_iter()
        .map(|module| (module.module_name, module.exports))
        .collect::<std::collections::HashMap<_, _>>();

    let source = std::io::stdin()
        .lines()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap()
        .join("\n");
    let source_name = std::env::args()
        .nth(1)
        .unwrap_or(String::from("test.ditto"));
    let module = parse_and_check_module(source, source_name, &everything);

    serde_json::to_writer_pretty(std::io::stdout(), &module).unwrap();
}
