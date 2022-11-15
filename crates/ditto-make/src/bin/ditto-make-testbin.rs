fn main() -> std::io::Result<()> {
    let ditto_sources = ditto_make::find_ditto_files("./ditto-src")?;
    let sources = ditto_make::Sources {
        config: std::path::PathBuf::from(ditto_config::CONFIG_FILE_NAME),
        ditto: ditto_sources,
    };
    let mut package_sources = ditto_make::PackageSources::new();
    if std::path::PathBuf::from("dep").exists() {
        let dep_ditto_sources = ditto_make::find_ditto_files("./dep/ditto-src")?;
        let dep_sources = ditto_make::Sources {
            config: ["dep", "ditto.toml"].iter().collect(),
            ditto: dep_ditto_sources,
        };
        package_sources.insert(
            ditto_config::PackageName::new_unchecked("dep".into()),
            dep_sources,
        );
    }
    let result = generate_build_ninja(sources, package_sources)
        .map(|(build_ninja, _get_warnings)| build_ninja);

    match result {
        Ok(build_ninja) => {
            println!("{}", build_ninja.into_syntax_path_slash());
        }
        Err(report) => {
            eprintln!("{:?}", report);
        }
    }
    Ok(())
}

fn generate_build_ninja(
    sources: ditto_make::Sources,
    package_sources: ditto_make::PackageSources,
) -> miette::Result<(ditto_make::BuildNinja, ditto_make::GetWarnings)> {
    ditto_make::generate_build_ninja(
        std::path::PathBuf::from("builddir"),
        std::path::PathBuf::from("ditto"),
        &semver::Version::parse("0.0.0-test").unwrap(),
        "compile",
        sources,
        package_sources,
    )
}
