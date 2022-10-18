use crate::{common, ninja::get_ninja_exe, pkg, spinner::Spinner, version::Version};
use clap::{Arg, ArgMatches, Command};
use console::Style;
use ditto_config::{read_config, Config, PackageName, CONFIG_FILE_NAME};
use ditto_make::{self as make, BuildNinja, GetWarnings, PackageSources, Sources};
use fs2::FileExt;
use log::{debug, trace};
use miette::{IntoDiagnostic, Result, WrapErr};
use notify::Watcher;
use std::{
    collections::HashMap,
    env::current_exe,
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{self, ExitStatus, Stdio},
    time::Instant,
};

pub static COMPILE_SUBCOMMAND: &str = "compile";

pub fn command<'a>(name: &str) -> Command<'a> {
    Command::new(name)
        .about("Build a project")
        .arg(
            Arg::new("watch")
                .short('w')
                .long("watch")
                .help("Watch files for changes"),
        )
        .arg(
            Arg::new("no-tests")
                .long("no-tests")
                .help("Ignore test modules and dependencies"),
        )
        .arg(
            Arg::new("execs")
                .long("exec")
                .help("Shell command to run on success")
                .takes_value(true)
                .multiple_occurrences(true),
        )
        // Useful for debugging why watches are/aren't triggering.
        // Should remove it eventually.
        .arg(Arg::new("debug-watcher").long("debug-watcher").hide(true))
}

pub async fn run(matches: &ArgMatches, ditto_version: &Version) -> Result<()> {
    // Read the ditto.toml immediately,
    // failing early if it's not present.
    let config_path: PathBuf = [".", CONFIG_FILE_NAME].iter().collect();
    let config = read_config(&config_path)?;

    if matches.is_present("watch") {
        run_watch(matches, ditto_version, &config_path, config).await
    } else {
        let what_happened = run_once(matches, ditto_version, &config_path, &config, true).await?;
        if !what_happened.is_error() {
            run_execs(matches)
        }
        what_happened.exit()
    }
}

pub async fn run_watch(
    matches: &ArgMatches,
    ditto_version: &Version,
    config_path: &Path,
    mut config: Config,
) -> Result<()> {
    let (event_sender, event_receiver) = crossbeam_channel::unbounded();
    let mut watcher = notify::RecommendedWatcher::new(event_sender).into_diagnostic()?;

    // Watch ditto.toml, src/** and test/**
    //
    // NOTE not watching packages as that seems wasteful...
    // Package source isn't going to be touched the majority of the time?
    // We could consider watching packages that are symlinks (i.e. local)

    // watch `./ditto.toml`
    watcher
        .watch(
            &PathBuf::from(CONFIG_FILE_NAME),
            notify::RecursiveMode::NonRecursive,
        )
        .into_diagnostic()?;

    // watch `./src` (should really be present)
    watcher
        .watch(&config.src_dir, notify::RecursiveMode::Recursive)
        .into_diagnostic()?;

    // watch `./tests` (if it's present)
    if config.test_dir.exists() {
        watcher
            .watch(&config.test_dir, notify::RecursiveMode::Recursive)
            .into_diagnostic()?;
    }

    // TODO: allow watching more files via config or a flag?

    let (run_sender, run_receiver) = crossbeam_channel::bounded::<(Config, bool)>(1);
    let (done_sender, done_receiver) = crossbeam_channel::bounded::<()>(1);

    // Clone unchanging things that need to be moved into the new thread
    let matches_clone = matches.clone();
    let ditto_version_clone = ditto_version.clone();
    let config_path_clone = config_path.to_path_buf();

    tokio::spawn(async move {
        loop {
            let (config, install_packages) = run_receiver.recv().unwrap();
            run_once_watch(
                &matches_clone,
                &ditto_version_clone,
                &config_path_clone,
                &config,
                install_packages,
            )
            .await;
            done_sender.send(()).unwrap();
        }
    });

    run_sender.send((config.clone(), true)).unwrap();
    //                               ^^^^
    //              Check packages are up to date on the first run

    // Track whether there is a run in progress (if so, we ignore notify events).
    // This is true initially due to the previous line ☝️
    let mut run_in_progress = true;

    // NOTE: this closure is a thing mostly because rustfmt doesn't handle
    // the `crossbeam_channel::select` macro, so I don't want to have much
    // logic there.
    type EventResult = std::result::Result<notify::Event, notify::Error>;
    let mut handle_event_result = |event: EventResult| -> bool {
        match event {
            Ok(ref event) if should_run_for_event(event) => {
                if matches.is_present("debug-watcher") {
                    dbg!(event);
                }

                let config_file_changed = event
                    .paths
                    .iter()
                    .flat_map(|path| path.extension().and_then(|ext| ext.to_str()))
                    .any(|ext| ext == "toml");

                // If the config file was touched,
                // then update the `config` value..
                if config_file_changed {
                    match read_config(config_path) {
                        Ok(latest_config) => {
                            config = latest_config;
                        }
                        Err(err) => {
                            eprintln!("{:?}", err);
                            return false;
                        }
                    }
                }

                run_sender
                    .send((
                        config.clone(),
                        // Check packages are up to date if the ditto.toml was touched
                        config_file_changed,
                    ))
                    .unwrap();

                true // return that a new run was started
            }
            other => {
                log::trace!("Ignoring notify event: {:?}", other);

                false // return that no new run was started
            }
        }
    };

    loop {
        crossbeam_channel::select! {
            recv(done_receiver) -> _ => {
                // Done!
                run_in_progress = false
            },
            recv(event_receiver) -> receive_result => {
                if let Ok(event_result) = receive_result {
                    if !run_in_progress {
                        let run_started = handle_event_result(event_result);
                        if run_started {
                            run_in_progress = true;
                        }
                    }
                }
            },
        }
    }

    fn should_run_for_event(event: &notify::Event) -> bool {
        use notify::{event::ModifyKind, EventKind};
        let event_kind_is_interesting = matches!(
            event.kind,
            EventKind::Modify(ModifyKind::Data(_)) | EventKind::Create(_) | EventKind::Remove(_)
        );

        if !event_kind_is_interesting {
            return false;
        }

        // Be selective about what we re-run for.
        // I.e. don't re-run for foreign files etc.
        let mut event_path_extensions = event
            .paths
            .iter()
            .flat_map(|path| path.extension().and_then(|ext| ext.to_str()));
        event_path_extensions.any(|ext| matches!(ext, "toml" | "ditto"))
    }

    async fn run_once_watch(
        matches: &ArgMatches,
        ditto_version: &Version,
        config_path: &Path,
        config: &Config,
        install_packages: bool,
    ) {
        if !matches.is_present("debug-watcher") {
            if let Err(_err) = clearscreen::clear() {
                // doesn't matter, let it fail?
            }
        }

        let result = run_once(
            matches,
            ditto_version,
            config_path,
            config,
            install_packages,
        )
        .await;
        match result {
            Err(err) => {
                // print the error but don't exit!
                eprintln!("{:?}", err);
            }
            Ok(what_happened) => {
                if what_happened.is_error() {
                    // If there was an error, stop here.
                    return;
                }

                // Print a "finished" message if no warnings were printed
                match what_happened {
                    WhatHappened::Nothing {
                        warnings_printed: false,
                        ..
                    } => {
                        println!("{}", Style::new().white().dim().apply_to("Nothing to do"));
                    }
                    WhatHappened::Success {
                        warnings_printed: false,
                    } => {
                        println!("{}", Style::new().green().bold().apply_to("All good!"));
                    }
                    _ => {
                        // Don't print anything, as
                    }
                }

                // Run shell hooks
                run_execs(matches);
            }
        }
    }
}

enum WhatHappened {
    /// "ninja: no work to do"
    ///
    /// Warnings may have been printed, though.
    Nothing {
        ninja_exit_status: ExitStatus,
        warnings_printed: bool,
    },
    /// Ninja had a non-zero exit status.
    ///
    /// Errors will have been printed.
    Error { ninja_exit_status: ExitStatus },
    /// Ninja ran successfully. Warnings may have been printed.
    Success { warnings_printed: bool },
}

impl WhatHappened {
    fn exit(self) -> ! {
        match self {
            Self::Nothing {
                ninja_exit_status, ..
            } => process::exit(ninja_exit_status.code().unwrap_or(0)),
            Self::Error {
                ninja_exit_status, ..
            } => process::exit(ninja_exit_status.code().unwrap_or(0)),
            Self::Success { .. } => process::exit(0),
        }
    }
    fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }
}

/// If successful returns the exit status of `ninja` and whether anything actually happened.
async fn run_once(
    matches: &ArgMatches,
    ditto_version: &Version,
    config_path: &Path,
    config: &Config,
    install_packages: bool,
) -> Result<WhatHappened> {
    // Need to acquire a lock on the build directory as lots of `ditto make`
    // processes running concurrently will cause problems!
    let lock = acquire_lock(config)?;
    debug!("Lock acquired");

    let include_test_stuff = !matches.is_present("no-tests");

    // Install/remove packages as needed
    // (this is a nicer pattern than requiring a run of a separate CLI command, IMO)
    if install_packages && !config.dependencies.is_empty() {
        pkg::check_packages_up_to_date(config, include_test_stuff)
            .await
            .wrap_err("error checking packages are up to date")?;
    }

    let now = Instant::now(); // for timing

    // Do the thing
    let result = make(config_path, config, ditto_version, include_test_stuff).await;

    lock.unlock()
        // Crash if we fail to release the lock otherwise things are likely to misbehave...
        .expect("Error releasing lock on build directory");

    debug!("make ran in {}ms", now.elapsed().as_millis());

    result
}

fn run_execs(matches: &ArgMatches) {
    if let Some(mut execs) = matches.values_of("execs") {
        while let Some(exec) = execs.next() {
            if let Some(shell_words) = shlex::split(exec) {
                if let Some((program, args)) = shell_words.split_first() {
                    print_feedback(format!("running {:?}", exec));
                    let result = process::Command::new(program).args(args).status();
                    match result {
                        Ok(exit_status) => {
                            if !exit_status.success() {
                                print_error(format!(
                                    "non-zero exit from {:?}, {}",
                                    exec, exit_status
                                ));
                                // Stop there, multiple `--exec` flags are effectively
                                // `&&` together
                                if execs.next().is_some() {
                                    print_error("stopping there".to_string());
                                }
                                return;
                            }
                        }
                        Err(err) => {
                            print_error(format!("EXEC ERROR {}", err));
                            // Stop there, multiple `--exec` flags are effectively
                            // `&&` together
                            if execs.next().is_some() {
                                print_error("stopping there".to_string());
                            }
                            return;
                        }
                    }
                } else {
                    print_feedback(format!("don't know how to execute {:?}, skipping", exec));
                }
            } else {
                unreachable!("Unexpected `None` value from shlex for {:?}", exec);
            }
        }
    }

    fn print_feedback(message: String) {
        if common::is_plain() {
            println!("{}", message);
        } else {
            println!("{}", Style::new().yellow().apply_to(message))
        }
    }

    fn print_error(message: String) {
        if common::is_plain() {
            eprintln!("{}", message);
        } else {
            eprintln!("{}", Style::new().red().bold().apply_to(message))
        }
    }
}

/// If successful returns the exit status of `ninja` and whether anything actually happened.
async fn make(
    config_path: &Path,
    config: &Config,
    ditto_version: &Version,
    include_test_sources: bool,
) -> Result<WhatHappened> {
    let (build_ninja, get_warnings) =
        generate_build_ninja(config_path, config, ditto_version, include_test_sources).map_err(
            |err| {
                // This is a bit brittle, but we want parse errors encountered during
                // build planning to be indistinguishable from parse errors encountered
                // during the actual build
                if err.root_cause().to_string() == "syntax error" {
                    //                                  ^^ BEWARE relying on this string is brittle!
                    err
                } else {
                    err.wrap_err("error generating build.ninja")
                }
            },
        )?;

    trace!("build.ninja generated");

    let mut build_ninja_path = config.ditto_dir.to_path_buf();
    build_ninja_path.push("build");
    build_ninja_path.set_extension("ninja");

    {
        if !config.ditto_dir.exists() {
            fs::create_dir_all(&config.ditto_dir)
                .into_diagnostic()
                .wrap_err(format!(
                    "error creating {}",
                    config.ditto_dir.to_string_lossy()
                ))?;
        }

        let mut handle = fs::File::create(&build_ninja_path)
            .into_diagnostic()
            .wrap_err(format!(
                "error creating ninja build file: {:?}",
                build_ninja_path.to_string_lossy()
            ))?;

        handle
            .write_all(build_ninja.into_syntax().as_bytes())
            .into_diagnostic()
            .wrap_err(format!(
                "error writing {:?}",
                build_ninja_path.to_string_lossy()
            ))?;

        debug!(
            "build.ninja written to {:?}",
            build_ninja_path.to_string_lossy()
        );
    }

    static NINJA_STATUS_MESSAGE: &str = "__NINJA";

    let ninja_exe = get_ninja_exe().await?;
    let mut child = process::Command::new(&ninja_exe)
        .arg("-f")
        .arg(&build_ninja_path)
        .stdout(Stdio::piped())
        // Mark ninja status messages so we can push them to our own progress spinner
        .env("NINJA_STATUS", NINJA_STATUS_MESSAGE)
        // Don't strip color codes, we'll handle that
        // https://github.com/ninja-build/ninja/commit/bf7107bb864d0383028202e3f4a4228c02302961
        .env("CLICOLOR_FORCE", "1")
        // Pass `is_plain` logic down to CLI calls made by ninja
        .env("DITTO_PLAIN", common::is_plain().to_string())
        .envs(if let Ok(log_dir) = std::env::var("DITTO_LOG_DIR") {
            vec![("DITTO_LOG_DIR", log_dir)]
        } else {
            vec![]
        })
        .spawn()
        .into_diagnostic()
        .wrap_err(format!(
            "error running ninja: {} -f {}",
            ninja_exe,
            build_ninja_path.to_string_lossy()
        ))?;

    let stdout = child.stdout.as_mut().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let mut stdout_lines = stdout_reader.lines();
    if let Some(Ok(first_line)) = stdout_lines.next() {
        // NOTE relying on the format of ninja messages like this could break
        // if DITTO_NINJA is set to a ninja version with a different format
        if first_line.starts_with("ninja: no work to do") {
            // Nothing to do,
            // still need to print warnings though
            let warnings = get_warnings()?;
            let warnings_printed = print_warnings(warnings);

            let ninja_exit_status = child
                .wait()
                .into_diagnostic()
                .wrap_err("ninja wasn't running?")?;

            return Ok(WhatHappened::Nothing {
                ninja_exit_status,
                warnings_printed,
            });
        } else {
            let mut spinner = Spinner::new();
            spinner.set_message(
                first_line
                    .trim_start_matches(NINJA_STATUS_MESSAGE)
                    .to_owned(),
            );

            // Our error/warning reports generally start with a blank line,
            // so we need to replicate that behavior when forwarding ninja
            // output for a consistent experience.
            let mut printed_initial_newline = false;
            while let Some(Ok(line)) = stdout_lines.next() {
                if line.starts_with(NINJA_STATUS_MESSAGE) {
                    spinner.set_message(line.trim_start_matches(NINJA_STATUS_MESSAGE).to_owned());
                } else if line.starts_with("ninja: build stopped: subcommand failed") {
                } else if console::strip_ansi_codes(&line).starts_with("FAILED") {
                    // The following line prints the command that was run (and failed)
                    // so swallow it
                    stdout_lines.next();
                } else {
                    if !printed_initial_newline {
                        spinner.println("\n");
                        printed_initial_newline = true
                    }
                    spinner.println(line);
                }
            }

            let ninja_exit_status = child.wait().expect("error waiting for ninja to exit");
            spinner.finish();
            if ninja_exit_status.success() {
                // Only print warnings if there wasn't an error
                let warnings = get_warnings()?;
                let warnings_printed = print_warnings(warnings);
                return Ok(WhatHappened::Success { warnings_printed });
            } else {
                return Ok(WhatHappened::Error { ninja_exit_status });
            }
        }
    } else {
        unreachable!()
    }

    fn print_warnings(warnings: Vec<miette::Report>) -> bool {
        if !warnings.is_empty() {
            let warnings_len = warnings.len();
            for (i, warning) in warnings.into_iter().enumerate() {
                if i == warnings_len - 1 {
                    eprintln!("{:?}", warning);
                } else {
                    eprint!("{:?}", warning);
                }
            }
            true
        } else {
            false
        }
    }
}

fn generate_build_ninja(
    config_path: &Path,
    config: &Config,
    ditto_version: &Version,
    include_test_sources: bool,
) -> Result<(BuildNinja, GetWarnings)> {
    let mut build_dir = config.ditto_dir.to_path_buf();
    build_dir.push("build");
    build_dir.push(&ditto_version.semversion.to_string());

    let ditto_bin = current_exe()
        .into_diagnostic()
        .wrap_err("error getting current executable")?;

    let mut ditto_files = find_ditto_files(&config.src_dir)?; // ditto-src
    if include_test_sources && config.test_dir.exists() {
        ditto_files.extend(find_ditto_files(&config.test_dir)?); // ditto-test
    }

    let sources = Sources {
        config: config_path.to_path_buf(),
        ditto: ditto_files,
    };

    let package_sources =
        get_package_sources(config).wrap_err("error finding ditto files in packages")?;

    make::generate_build_ninja(
        build_dir,
        ditto_bin,
        &ditto_version.semversion,
        COMPILE_SUBCOMMAND,
        sources,
        package_sources,
    )
}

fn get_package_sources(config: &Config) -> Result<PackageSources> {
    let mut package_sources = HashMap::new();
    for path in pkg::list_installed_packages(&pkg::mk_packages_dir(config))? {
        let package_name =
            PackageName::new_unchecked(path.file_name().unwrap().to_string_lossy().into_owned());
        let sources = get_sources_for_dir(&path)?;
        package_sources.insert(package_name, sources);
    }
    Ok(package_sources)
}

fn get_sources_for_dir(dir: &Path) -> Result<Sources> {
    let mut config_path = dir.to_path_buf();
    config_path.push(CONFIG_FILE_NAME);
    let config = read_config(&config_path)?;

    let mut src_dir = dir.to_path_buf();
    src_dir.push(config.src_dir);

    let ditto_sources = find_ditto_files(src_dir)?;
    Ok(Sources {
        config: config_path,
        ditto: ditto_sources,
    })
}

fn find_ditto_files<P: AsRef<Path>>(root: P) -> Result<Vec<PathBuf>> {
    make::find_ditto_files(root.as_ref())
        .into_diagnostic()
        .wrap_err(format!(
            "error finding ditto files in {}",
            root.as_ref().to_string_lossy()
        ))
}

static LOCK_FILE: &str = "_lock";

fn acquire_lock(config: &Config) -> Result<impl FileExt> {
    if !config.ditto_dir.exists() {
        debug!(
            "{} doesn't exist, creating",
            config.ditto_dir.to_string_lossy()
        );

        fs::create_dir_all(&config.ditto_dir)
            .into_diagnostic()
            .wrap_err(format!(
                "error creating {}",
                config.ditto_dir.to_string_lossy()
            ))?;
    }

    let mut lock_file = config.ditto_dir.to_path_buf();
    lock_file.push(LOCK_FILE);

    debug!("Opening lock file at {}", lock_file.to_string_lossy());
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&lock_file)
        .into_diagnostic()
        .wrap_err(format!(
            "error opening lock file {}",
            lock_file.to_string_lossy()
        ))?;

    if file.try_lock_exclusive().is_ok() {
        Ok(file)
    } else {
        println!("Waiting for lock...");
        file.lock_exclusive()
            .into_diagnostic()
            .wrap_err("error waiting for lock")?;
        Ok(file)
    }
}
