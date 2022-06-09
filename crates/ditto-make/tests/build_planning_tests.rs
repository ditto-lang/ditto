use lazy_static::lazy_static;

lazy_static! {
    static ref SERIAL_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

macro_rules! test_with_current_dir {
    ($dir:expr, $name: ident, $body: block) => {
        #[test]
        fn $name() -> std::io::Result<()> {
            let guard = SERIAL_TEST_MUTEX.lock().unwrap();

            let original_dir = std::env::current_dir()?;
            std::env::set_current_dir($dir)?;
            let result = std::panic::catch_unwind(|| $body);
            std::env::set_current_dir(original_dir)?;
            match result {
                Err(err) => {
                    drop(guard);
                    std::panic::resume_unwind(err);
                }
                Ok(result) => result,
            }
        }
    };
}

macro_rules! assert_build_ninja {
    ($dir:expr, $name:ident) => {
        test_with_current_dir!($dir, $name, {
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
            let (build_file, _) = generate_build_ninja(sources, package_sources).unwrap();
            let want = std::fs::read_to_string("./build.ninja")?;
            let got = build_file.into_syntax_path_slash();
            similar_asserts::assert_str_eq!(got: got, want: want);
            Ok(())
        });
    };
}

macro_rules! assert_build_ninja_error {
    ($dir:expr, $name:ident, $error_string:expr) => {
        test_with_current_dir!($dir, $name, {
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
            let err = generate_build_ninja(sources, package_sources)
                .map(|(build_ninja, _)| build_ninja)
                .unwrap_err();
            similar_asserts::assert_str_eq!(got: err.to_string(), want: $error_string);
            Ok(())
        });
    };
}

assert_build_ninja!("./fixtures/all-good", builds_a_javascript_project);
assert_build_ninja!("./fixtures/missing-module", it_ignores_bad_imports);
assert_build_ninja!("./fixtures/no-codegen", it_works_without_targets);

assert_build_ninja_error!(
    "./fixtures/target-mismatch",
    it_fails_for_unsupported_targets,
    "package \"dep\" doesn't support targets: \"web\""
);
assert_build_ninja_error!(
    "./fixtures/unsupported-ditto-version",
    it_fails_for_unsupported_ditto_version,
    "ditto version requirement not met for current_package: current version = 0.0.0-test, wanted = ^1.0.0"
);
assert_build_ninja_error!(
    "./fixtures/duplicate-module-name",
    it_fails_for_duplicate_module_names,
    "module name `A` is taken"
);
assert_build_ninja_error!(
    "./fixtures/module-cycle",
    it_fails_for_module_cycles,
    "modules form a cycle: `A`, `B`"
);
assert_build_ninja_error!(
    "./fixtures/self-referencing-module",
    it_fails_for_self_referencing_modules,
    "module `A` can't import itself!"
);

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
