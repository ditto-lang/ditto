// Maybe this should live in it's own crate?
use crate::{
    common::{get_ditto_cache_dir, is_plain},
    spinner::Spinner,
};
use console::{Emoji, Style};
use ditto_config::{
    read_config, Config, Dependencies, GithubPackageSpec, PackageName,
    PackageSetPackages as Packages, PackageSpec, CONFIG_FILE_NAME,
};
use indicatif::MultiProgress;
use log::{debug, warn};
use miette::{miette, IntoDiagnostic, Result, WrapErr};
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    ffi::OsStr,
    fs,
    hash::{Hash, Hasher},
    io::BufReader,
    path::{Path, PathBuf},
};

pub async fn check_packages_up_to_date(
    config: &Config,
    include_test_dependencies: bool,
) -> Result<()> {
    debug!("Checking if packages are up to date");

    let available_packages = config.resolve_packages().await?.clone();
    let want_hash = hash_packages_inputs(&config.dependencies, &available_packages);
    debug!("Current hash is: {}", want_hash);

    let packages_dir = get_or_create_packages_dir(config)?;
    let hash_file = mk_hash_file(&packages_dir);

    if hash_file.exists() {
        let got_hash_string = fs::read_to_string(&hash_file)
            .into_diagnostic()
            .wrap_err("error reading packages hash file")?;

        debug!(
            "Last hash ({:?}) was {}",
            hash_file.as_os_str(),
            got_hash_string
        );

        if let Ok(got_hash) = got_hash_string.parse::<u64>() {
            if want_hash == got_hash {
                debug!("Packages are up to date");
                return Ok(());
            }
        } else {
            warn!("Corrupted hash file? got {}", got_hash_string);
        }
    };

    debug!("Updating packages");
    if is_plain() {
        println!("Updating packages...");
    } else {
        println!(
            "{}{}",
            Emoji::new("📦 ", ""),
            Style::new().cyan().apply_to("Updating packages...")
        );
    }

    let installed_packages = get_installed_packages(&packages_dir)?;

    let mut dependencies = config.dependencies.clone();
    if include_test_dependencies {
        dependencies.extend(config.test_dependencies.clone());
    }

    let mut multi_progress = MultiProgress::new();
    update_dependencies(
        &mut multi_progress,
        &packages_dir,
        &dependencies,
        &mut Dependencies::new(),
        &installed_packages,
        &available_packages,
    )
    .await?;

    debug!(
        "Updating {} with {}",
        hash_file.to_string_lossy(),
        want_hash
    );
    fs::write(hash_file, want_hash.to_string().as_bytes()).into_diagnostic()?;
    Ok(())
}

fn hash_packages_inputs(dependencies: &Dependencies, packages: &Packages) -> u64 {
    let mut dependencies = dependencies.iter().cloned().collect::<Vec<_>>();

    let mut packages = packages
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<Vec<_>>();

    // REVIEW do we need to sort?
    dependencies.sort();
    packages.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = DefaultHasher::new();

    (dependencies, packages).hash(&mut hasher);
    hasher.finish()
}

#[async_recursion::async_recursion]
async fn update_dependencies(
    _multi_progress: &mut MultiProgress,
    packages_dir: &Path,
    dependencies: &Dependencies,
    updated_dependencies: &mut Dependencies,
    installed_packages: &Packages,
    available_packages: &Packages,
) -> Result<()> {
    for dependency in dependencies {
        if updated_dependencies.contains(dependency) {
            continue;
        }
        match (
            installed_packages.get(dependency),
            available_packages.get(dependency),
        ) {
            (Some(installed_spec), Some(available_spec)) => {
                if *installed_spec != *available_spec {
                    // Specs differ, update

                    //let mut progress = multi_progress.add(ProgressBar::new_spinner());
                    let mut spinner = Spinner::new_with_prefix(dependency.as_str().to_string());
                    debug!("Removing existing install of {}", dependency.as_str());
                    spinner.set_message("removing existing install");
                    remove_package(packages_dir, dependency)?;
                    install_package(spinner, packages_dir, dependency, available_spec).await?;
                }
                updated_dependencies.insert(dependency.clone());
                let config = read_package_config(packages_dir, dependency)?;
                update_dependencies(
                    _multi_progress,
                    packages_dir,
                    &config.dependencies,
                    updated_dependencies,
                    installed_packages,
                    available_packages,
                )
                .await?
            }
            (None, Some(available_spec)) => {
                // Not installed

                //let mut progress = multi_progress.add(ProgressBar::new_spinner());
                let spinner = Spinner::new_with_prefix(dependency.as_str().to_string());

                install_package(spinner, packages_dir, dependency, available_spec).await?;
                updated_dependencies.insert(dependency.clone());
                let config = read_package_config(packages_dir, dependency)?;
                update_dependencies(
                    _multi_progress,
                    packages_dir,
                    &config.dependencies,
                    updated_dependencies,
                    installed_packages,
                    available_packages,
                )
                .await?
            }
            (Some(_installed_spec), None) => {
                return Err(miette!(
                    "{:?} package installed, but no longer in the package set?",
                    dependency
                ));
            }
            (None, None) => {
                return Err(miette!("{:?} not available in the package set", dependency));
            }
        }
    }
    Ok(())
}

const EXTENSION_SPEC: &str = "spec";

async fn install_package(
    spinner: Spinner,
    packages_dir: &Path,
    package_name: &str,
    spec: &PackageSpec,
) -> Result<()> {
    debug!("installing {:?}", package_name);
    match spec {
        PackageSpec::Path { path } => {
            install_package_from_path(spinner, packages_dir, package_name, path)?;
        }
        PackageSpec::Github {
            github,
            revision,
            sha256,
        } => {
            install_package_from_github(
                spinner,
                packages_dir,
                package_name,
                github,
                revision,
                sha256,
            )
            .await?;
        }
    }

    let mut spec_path = packages_dir.to_path_buf();
    spec_path.push(package_name);
    spec_path.set_extension(EXTENSION_SPEC);
    let spec_file = fs::File::create(&spec_path).into_diagnostic()?;
    serde_json::to_writer(spec_file, spec).into_diagnostic()?;

    debug!(
        "{:?} spec written to {}",
        package_name,
        spec_path.to_string_lossy()
    );

    Ok(())
}

fn install_package_from_path(
    mut spinner: Spinner,
    packages_dir: &Path,
    package_name: &str,
    path: &Path,
) -> Result<()> {
    let src = pathdiff::diff_paths(path, packages_dir).unwrap();
    let mut dst = packages_dir.to_path_buf();
    dst.push(package_name);

    debug!(
        "linking {} -> {}",
        dst.to_string_lossy(),
        src.to_string_lossy(),
    );
    spinner.set_message(format!(
        "{} -> {}",
        dst.to_string_lossy(),
        src.to_string_lossy(),
    ));
    symlink::symlink_dir(src, dst).into_diagnostic()?;

    spinner.success("installed");

    Ok(())
}

async fn install_package_from_github(
    mut spinner: Spinner,
    packages_dir: &Path,
    package_name: &str,
    github: &GithubPackageSpec,
    revision: &str,
    sha256: &str,
) -> Result<()> {
    let mut zip_archive =
        fetch_cached_github_archive(&mut spinner, github, revision, sha256).await?;

    let mut dst = packages_dir.to_path_buf();
    dst.push(package_name);

    for name in zip_archive
        .file_names()
        .map(|str| str.to_string())
        .collect::<Vec<_>>()
    {
        let mut zip_file = zip_archive.by_name(&name).into_diagnostic()?;
        if zip_file.is_dir() {
            continue;
        }
        let zip_path = zip_file
            .enclosed_name()
            .ok_or_else(|| miette!("encountered bad zip file path!"))?;
        let mut dst = dst.clone();
        dst.extend(
            // drop the root directory from the zip file
            zip_path.components().skip(1),
        );

        if let Some(parent) = dst.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).into_diagnostic()?;
            }
        }
        debug!("{:?} -> {:?}", zip_path, &dst);
        let mut file = std::fs::File::create(dst).into_diagnostic()?;
        std::io::copy(&mut zip_file, &mut file).into_diagnostic()?;
    }

    spinner.success("installed");
    Ok(())
}

async fn fetch_cached_github_archive(
    spinner: &mut Spinner,
    github: &GithubPackageSpec,
    revision: &str,
    sha256: &str,
) -> Result<zip::ZipArchive<std::fs::File>> {
    // Is it in the cache?
    let mut cached_zip = get_github_cache_dir()?;
    cached_zip.push(sha256);
    cached_zip.set_extension("zip");
    if cached_zip.exists() {
        let file = std::fs::File::open(&cached_zip).into_diagnostic()?;
        let zip_archive = zip::ZipArchive::new(file).into_diagnostic()?;
        return Ok(zip_archive);
    }

    // Nope, download it...
    let archive_url = format!(
        "https://github.com/{owner}/{repo}/archive/{revision}.zip",
        owner = github.owner,
        repo = github.repo,
    );
    debug!("GET {}", archive_url);
    spinner.set_message(format!("Downloading {}", archive_url));
    let response = reqwest::get(&archive_url).await.into_diagnostic()?;
    if !response.status().is_success() {
        return Err(miette!("{} {}", response.status(), archive_url));
    }
    let content = response.bytes().await.into_diagnostic()?;
    let got_sha256 = sha256::digest(content.as_ref());
    if got_sha256 != sha256 {
        return Err(miette!(
            "sha256 mismatch for {}/{}, expected {:?} but got {:?}",
            github.owner,
            github.repo,
            sha256,
            got_sha256
        ));
    }
    debug!("Writing archive to {:?}", cached_zip);
    std::fs::write(&cached_zip, content).into_diagnostic()?;
    let file = std::fs::File::open(&cached_zip).into_diagnostic()?;
    let zip_archive = zip::ZipArchive::new(file).into_diagnostic()?;
    Ok(zip_archive)
}

pub fn get_github_cache_dir() -> Result<PathBuf> {
    let mut cache_dir = get_ditto_cache_dir()?;
    cache_dir.push("github");
    if !cache_dir.exists() {
        debug!(
            "Github cache directory doesn't exist, creating {:?}",
            cache_dir
        );
        std::fs::create_dir_all(&cache_dir)
            .into_diagnostic()
            .wrap_err(format!(
                "error initializing github cache dir at {:?}",
                cache_dir
            ))?;
    }

    Ok(cache_dir)
}

fn remove_package(packages_dir: &Path, package_name: &str) -> Result<()> {
    debug!("Removing package {:?}", package_name);
    for result in fs::read_dir(packages_dir).into_diagnostic()? {
        let entry = result.into_diagnostic()?;
        if entry.path().starts_with(package_name) {
            remove_dir_entry(entry)?;
        }
    }
    Ok(())
}

fn read_package_config(packages_dir: &Path, package_name: &str) -> Result<Config> {
    let mut package_config_path = packages_dir.to_path_buf();
    package_config_path.push(package_name);
    package_config_path.push(CONFIG_FILE_NAME);
    read_config(package_config_path)
}

fn get_installed_packages(packages_dir: &Path) -> Result<Packages> {
    let package_entries = fs::read_dir(packages_dir)
        .into_diagnostic()?
        .into_iter()
        .map(|result| result.into_diagnostic())
        .collect::<Result<Vec<_>>>()?;

    let mut installed = Packages::new();
    let mut legit = HashSet::new();
    for entry in package_entries.iter() {
        let path = entry.path();
        let extension_str = path.extension().and_then(|os_str| os_str.to_str());
        if let Some(EXTENSION_SPEC) = extension_str {
            let spec_path = path;
            let package_name = spec_path.file_stem().unwrap(); // should really be there
            let package_path = package_entries.iter().find_map(|entry| {
                let path = entry.path();
                if path.file_name() == Some(package_name) {
                    Some(path)
                } else {
                    None
                }
            });
            if let Some(package_path) = package_path {
                let file = fs::File::open(&spec_path).into_diagnostic()?;
                let reader = BufReader::new(file);
                let package_spec = serde_json::from_reader(reader).into_diagnostic()?;
                installed.insert(
                    PackageName::new_unchecked(package_name.to_string_lossy().into_owned()),
                    package_spec,
                );
                legit.insert(spec_path);
                legit.insert(package_path);
            }
        }
    }

    // Tidy up
    for entry in package_entries {
        if !legit.contains(&entry.path()) {
            remove_dir_entry(entry)?
        }
    }

    Ok(installed)
}

pub fn list_installed_packages(packages_dir: &Path) -> Result<Vec<PathBuf>> {
    if !packages_dir.exists() {
        return Ok(vec![]);
    }
    let package_entries = fs::read_dir(packages_dir)
        .into_diagnostic()
        .wrap_err(format!(
            "error reading packages directory {}",
            packages_dir.to_string_lossy()
        ))?
        .into_iter()
        .map(|result| {
            result
                .into_diagnostic()
                .wrap_err("error getting packages directory entry")
        })
        .collect::<Result<Vec<_>>>()?;

    let mut installed = Vec::new();
    for entry in package_entries {
        let path = entry.path();
        if path.file_name() == Some(OsStr::new(HASH_FILE)) {
            continue;
        }
        if path.extension() == Some(OsStr::new(EXTENSION_SPEC)) {
            continue;
        }
        installed.push(path);
    }
    Ok(installed)
}

fn remove_dir_entry(entry: fs::DirEntry) -> Result<()> {
    let path = entry.path();
    let file_type = entry.file_type().into_diagnostic()?;
    if file_type.is_dir() {
        debug!("Removing directory: {}", path.to_string_lossy());
        fs::remove_dir_all(path).into_diagnostic()
    } else {
        debug!("Removing file: {}", path.to_string_lossy());
        fs::remove_file(path).into_diagnostic()
    }
}

static HASH_FILE: &str = "_hash";

fn mk_hash_file(packages_dir: &Path) -> PathBuf {
    let mut path = packages_dir.to_path_buf();
    path.push(HASH_FILE);
    path
}

fn get_or_create_packages_dir(config: &Config) -> Result<PathBuf> {
    let path = mk_packages_dir(config);
    if !path.exists() {
        debug!("{} doesn't exist, creating", path.to_string_lossy());

        fs::create_dir_all(&path)
            .into_diagnostic()
            .wrap_err(format!(
                "error creating ditto packages dir: {:?}",
                path.as_os_str()
            ))?;
    }
    Ok(path)
}

pub fn mk_packages_dir(config: &Config) -> PathBuf {
    let mut path = config.ditto_dir.to_path_buf();
    path.push("packages");
    path
}
