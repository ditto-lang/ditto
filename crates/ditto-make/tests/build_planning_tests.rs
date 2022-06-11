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
        assert_build_ninja!($dir, $name, None);
    };
    ($dir:expr, $name:ident, $docs_dir:expr) => {
        test_with_current_dir!($dir, $name, {
            let (sources, package_sources) = collect_sources()?;
            let (build_file, _) =
                generate_build_ninja(sources, package_sources, $docs_dir).unwrap();
            let want = std::fs::read_to_string("./build.ninja")?;
            let got = build_file.into_syntax_path_slash();
            similar_asserts::assert_eq!(got: got, want: want);
            Ok(())
        });
    };
}

macro_rules! assert_build_ninja_error {
    ($dir:expr, $name:ident, $error_string:expr) => {
        assert_build_ninja_error!($dir, $name, $error_string, None);
    };
    ($dir:expr, $name:ident, $error_string:expr, $docs_dir:expr) => {
        test_with_current_dir!($dir, $name, {
            let (sources, package_sources) = collect_sources()?;
            let err = generate_build_ninja(sources, package_sources, $docs_dir)
                .map(|(build_ninja, _)| build_ninja)
                .unwrap_err();
            similar_asserts::assert_eq!(got: err.to_string(), want: $error_string);
            Ok(())
        });
    };
}

assert_build_ninja!("./fixtures/all-good", builds_a_javascript_project);
assert_build_ninja!("./fixtures/missing-module", it_ignores_bad_imports);
assert_build_ninja!("./fixtures/no-codegen", it_works_without_targets);
assert_build_ninja!("./fixtures/with-docs", it_generates_docs, Some("docs"));

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

fn collect_sources() -> std::io::Result<(ditto_make::Sources, ditto_make::PackageSources)> {
    static SRC_DIR: &str = "ditto-src";
    static TEST_DIR: &str = "ditto-test";

    let mut source_files = ditto_make::find_ditto_source_files(SRC_DIR, true)?;
    if std::path::Path::new(TEST_DIR).exists() {
        source_files.extend(ditto_make::find_ditto_source_files(TEST_DIR, false)?);
    }
    let sources = ditto_make::Sources {
        config: std::path::PathBuf::from(ditto_config::CONFIG_FILE_NAME),
        source_files,
    };

    static PACKAGE_DEP: &str = "dep";

    let mut package_sources = ditto_make::PackageSources::new();
    let dep_dir = std::path::PathBuf::from(PACKAGE_DEP);
    if dep_dir.exists() {
        let mut dep_src_dir = dep_dir.clone();
        dep_src_dir.push(SRC_DIR);

        let mut dep_source_files = ditto_make::find_ditto_source_files(dep_src_dir, true)?;

        let mut dep_test_dir = dep_dir;
        dep_test_dir.push(TEST_DIR);
        if dep_test_dir.exists() {
            dep_source_files.extend(ditto_make::find_ditto_source_files(dep_test_dir, false)?);
        }
        let dep_sources = ditto_make::Sources {
            config: ["dep", "ditto.toml"].iter().collect(),
            source_files: dep_source_files,
        };
        package_sources.insert(
            ditto_config::PackageName::new_unchecked(PACKAGE_DEP.into()),
            dep_sources,
        );
    }
    Ok((sources, package_sources))
}

fn generate_build_ninja(
    sources: ditto_make::Sources,
    package_sources: ditto_make::PackageSources,
    docs_dir: Option<&str>,
) -> miette::Result<(ditto_make::BuildNinja, ditto_make::GetWarnings)> {
    ditto_make::generate_build_ninja(
        std::path::PathBuf::from("builddir"),
        &std::path::Path::new("ditto"),
        &semver::Version::parse("0.0.0-test").unwrap(),
        "compile",
        sources,
        package_sources,
        docs_dir.map(|str| std::path::Path::new(str)),
    )
}
