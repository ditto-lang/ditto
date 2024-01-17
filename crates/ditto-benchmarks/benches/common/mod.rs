pub fn preamble(
    source: &'static str,
    module_exports: Vec<&'static str>,
) -> (
    ditto_cst::Module,
    ditto_ast::Module,
    ditto_checker::Everything,
    ditto_codegen_js::Config,
) {
    let cst_module = ditto_cst::Module::parse(source).unwrap();
    let mut everything = ditto_checker::Everything::default();
    for module_exports_json in module_exports {
        let (mn, exports): (ditto_ast::ModuleName, ditto_ast::ModuleExports) =
            serde_json::from_str(module_exports_json).unwrap();
        everything.modules.insert(mn, exports);
    }
    let (ast_module, warnings) =
        ditto_checker::check_module(&everything, cst_module.clone()).unwrap();

    assert!(warnings.is_empty());
    return (cst_module, ast_module, everything, codegen_js_config());
}

pub fn codegen_js_config() -> ditto_codegen_js::Config {
    ditto_codegen_js::Config {
        module_name_to_path: Box::new(|(package_name, module_name)| {
            let module_path = module_name
                .0
                .into_iter()
                .map(|proper_name| proper_name.0)
                .collect::<Vec<_>>()
                .join(".");

            match package_name {
                None => module_path,
                Some(ditto_ast::PackageName(pkg)) => format!("{}/{}", pkg, module_path),
            }
        }),
        foreign_module_path: "./foreign.js".into(),
    }
}
