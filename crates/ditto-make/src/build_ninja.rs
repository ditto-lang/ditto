use crate::{common, compile};
use ditto_ast as ast;
use ditto_config::{read_config, PackageName};
use ditto_cst as cst;
use miette::{bail, Diagnostic, IntoDiagnostic, NamedSource, Result, SourceSpan};
use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// A `.ditto` file.
pub struct SourceFile {
    /// Path to the `.ditto` file.
    path: PathBuf,
    /// Whether this module should have documentation generated for it.
    document: bool,
}

impl SourceFile {
    /// Create a [SourceFile].
    ///
    /// The module will be included in generated documentation
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            document: true,
        }
    }

    /// Create a [SourceFile], but ignore it in any generated documentation.
    pub fn without_docs(path: PathBuf) -> Self {
        Self {
            path,
            document: false,
        }
    }
}

/// A config file and a load of `*.ditto` files.
pub struct Sources {
    /// Path to the ditto config file.
    pub config: PathBuf,
    /// `*.ditto` files.
    pub source_files: Vec<SourceFile>,
}

/// [Sources] mapped to a package name.
pub type PackageSources = HashMap<PackageName, Sources>;

/// The type of function returned by [generate_build_ninja] that can be used to retrieve
/// compilation warnings.
pub type GetWarnings = impl FnOnce() -> Result<Vec<miette::Report>>;

/// Generates a [build.ninja](https://ninja-build.org/manual.html#_writing_your_own_ninja_files)
/// file and also returns a function for retrieving compiler warnings once `ninja` has run.
pub fn generate_build_ninja(
    build_dir: PathBuf,
    ditto_bin: &Path,
    ditto_version: &semver::Version,
    compile_subcommand: &'static str,
    sources: Sources,
    package_sources: PackageSources,
    docs_dir: Option<&Path>, // directory to generate docs to. If `None` then no docs will be generated.
) -> Result<(BuildNinja, GetWarnings)> {
    // TODO make this more concurrent!

    let config = read_config(&sources.config)?;

    // Initial build.ninja file, extended later
    let mut build_ninja = BuildNinja::new(&build_dir, ditto_bin, compile_subcommand);

    // Push extra build rules as needed
    if config.targets_js() {
        build_ninja
            .rules
            .push(Rule::new_js(ditto_bin, compile_subcommand));

        // We only generate package.json files for dependencies,
        // so if there are none then no need for the rule.
        if !package_sources.is_empty() {
            build_ninja
                .rules
                .push(Rule::new_package_json(ditto_bin, compile_subcommand));
        }
    }

    if docs_dir.is_some() {
        build_ninja
            .rules
            .push(Rule::new_doc_html_module(ditto_bin, compile_subcommand));
        build_ninja
            .rules
            .push(Rule::new_doc_html_index(ditto_bin, compile_subcommand));
    }

    let js_dirs = if config.targets_js() {
        let dist_dir = config.codegen_js_config.dist_dir;
        let packages_dir = config.codegen_js_config.packages_dir;
        build_ninja
            .builds
            .extend(package_sources.iter().map(|(package_name, sources)| {
                let mut package_json_path = packages_dir.clone();
                package_json_path.push(package_name.as_str());
                package_json_path.push("package.json");
                Build::new_package_json(package_name, package_json_path, sources.config.clone())
            }));
        Some((dist_dir, packages_dir))
    } else {
        None
    };

    let (graph, graph_nodes) = prepare_build_graph(sources, package_sources, ditto_version)?;

    // Paths to serialized warnings, so the caller can replay them
    let mut checker_warnings_paths: Vec<PathBuf> = Vec::new();

    // All the .ast-exports files which will be used as input to docs/index.html
    let mut doc_ast_exports_paths: Vec<PathBuf> = Vec::new();

    for (node_index, node) in graph_nodes.clone() {
        let node_string = node.to_string();
        let ast_path = mk_ast_path(
            build_dir.clone(),
            &node.package_name,
            &node.module_name,
            common::EXTENSION_AST,
        );

        let ast_exports_path = mk_ast_path(
            build_dir.clone(),
            &node.package_name,
            &node.module_name,
            common::EXTENSION_AST_EXPORTS,
        );

        let checker_warnings_path = if node.package_name.is_none() {
            let checker_warnings_path = mk_ast_path(
                build_dir.clone(),
                &node.package_name,
                &node.module_name,
                common::EXTENSION_CHECKER_WARNINGS,
            );
            checker_warnings_paths.push(checker_warnings_path.clone());
            Some(checker_warnings_path)
        } else {
            None
        };

        let dependency_ast_export_paths = graph
            .neighbors(node_index)
            .map(|idx| {
                let dep_node = graph_nodes.get(&idx).unwrap();
                mk_ast_path(
                    build_dir.clone(),
                    &dep_node.package_name,
                    &dep_node.module_name,
                    common::EXTENSION_AST_EXPORTS,
                )
            })
            .collect::<Vec<_>>();

        if let Some((ref dist_dir, ref packages_dir)) = js_dirs {
            let js_path = if let Some(package_name) = node.package_name {
                let mut js_path = packages_dir.clone();
                js_path.push(package_name.as_str());
                js_path.push(common::module_name_to_file_stem(node.module_name));
                js_path.set_extension(common::EXTENSION_JS);
                js_path
            } else {
                let mut js_path = dist_dir.clone();
                js_path.push(common::module_name_to_file_stem(node.module_name));
                js_path.set_extension(common::EXTENSION_JS);
                js_path
            };
            //
            build_ninja.builds.push(Build::new_js(
                node_string.clone(),
                js_path,
                ast_path.clone(),
            ));
        }

        // docs/Some.Module.html
        if let Some(docs_dir) = docs_dir {
            if node.document {
                let mut html_path = docs_dir.to_path_buf();
                html_path.push(&node_string);
                html_path.set_extension(common::EXTENSION_HTML);
                build_ninja.builds.push(Build::new_doc_html_module(
                    node_string.clone(),
                    html_path.to_path_buf(),
                    ast_exports_path.clone(),
                ));

                doc_ast_exports_paths.push(ast_exports_path.clone());
            }
        }

        build_ninja.builds.push(Build::new_ast(
            node_string,
            ast_path,
            ast_exports_path,
            checker_warnings_path,
            node.source_path,
            dependency_ast_export_paths,
        ));
    }

    // docs/index.html
    if let Some(docs_dir) = docs_dir {
        let mut index_html_path = docs_dir.to_path_buf();
        index_html_path.push("index");
        index_html_path.set_extension(common::EXTENSION_HTML);
        build_ninja.builds.push(Build::new_doc_html_index(
            index_html_path,
            doc_ast_exports_paths,
        ));
    }

    // Callback to get all warnings for the current package
    //               : GetWarnings
    let get_warnings = move || {
        let mut warnings = Vec::new();
        for warnings_path in checker_warnings_paths {
            let warnings_bundle =
                common::deserialize::<Option<compile::WarningsBundle>>(&warnings_path)?;

            if let Some(compile::WarningsBundle {
                name,
                source,
                warnings: warning_reports,
            }) = warnings_bundle
            {
                let source = std::sync::Arc::new(source);
                warnings.extend(warning_reports.into_iter().map(|warning_report| {
                    miette::Report::from(warning_report)
                        .with_source_code(miette::NamedSource::new(&name, source.clone()))
                }))
            }
        }
        Ok(warnings)
    };

    Ok((build_ninja, get_warnings))
}

fn mk_ast_path(
    mut base: PathBuf,
    package_name: &Option<PackageName>,
    module_name: &ast::ModuleName,
    extension: &str,
) -> PathBuf {
    if let Some(package_name) = package_name {
        base.push(package_name.as_str());
    }
    base.push(common::module_name_to_file_stem(module_name.clone()));
    base.set_extension(extension);
    base
}

// REVIEW do we need to duplicate the nodes like this?
type BuildGraph = petgraph::Graph<BuildGraphNode, &'static str>;
type BuildGraphNodes = HashMap<petgraph::graph::NodeIndex, BuildGraphNode>;

#[derive(Clone)]
struct BuildGraphNode {
    package_name: Option<PackageName>,
    module_name: ast::ModuleName,
    source_path: PathBuf,
    imports: Vec<cst::ImportLine>,
    document: bool,
}

impl fmt::Display for BuildGraphNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref package_name) = self.package_name {
            write!(f, "{}:", package_name.as_str())?;
        }
        write!(f, "{}", self.module_name)
    }
}

fn prepare_build_graph(
    sources: Sources,
    package_sources: PackageSources,
    ditto_version: &semver::Version,
) -> Result<(BuildGraph, BuildGraphNodes)> {
    let mut build_graph = BuildGraph::new();
    let mut build_graph_nodes = BuildGraphNodes::new();

    let current_config = read_config(&sources.config)?;

    let all_sources = package_sources
        .into_iter()
        .map(|(package_name, sources)| (Some(package_name), sources))
        .chain(std::iter::once((None, sources)));

    // Add the nodes
    for (package_name, sources) in all_sources {
        let config = if package_name.is_none() {
            current_config.clone()
        } else {
            read_config(sources.config)?
        };

        // Check ditto version requirement
        if let Some(required_ditto_version) = config.required_ditto_version {
            if !required_ditto_version.matches(ditto_version) {
                bail!(
                    "ditto version requirement not met for {}: current version = {}, wanted = {}",
                    package_name.map_or("current_package".into(), |package_name| format!(
                        "{:?}",
                        package_name
                    )),
                    ditto_version,
                    required_ditto_version
                );
            }
        }

        // Check target compatibility
        if let Some(ref package_name) = package_name {
            if !current_config.targets.is_subset(&config.targets) {
                let unsupported = current_config
                    .targets
                    .difference(&config.targets)
                    .map(|target| serde_json::to_string(target).unwrap())
                    .collect::<Vec<_>>()
                    .join(", ");
                bail!(
                    "package {:?} doesn't support targets: {}",
                    package_name.as_str(),
                    unsupported
                );
            }
        }

        // Check for duplicate module names
        #[derive(Error, Debug, Diagnostic)]
        #[error("module name `{module_name}` is taken")]
        struct DuplicateModuleError {
            #[source_code]
            input: NamedSource,

            module_name: String,

            #[label("module name is used by {other_file}")]
            module_name_span: SourceSpan,

            other_file: String,
        }
        let mut module_names_seen: HashMap<ast::ModuleName, PathBuf> = HashMap::new();

        // TODO make this more async?
        for source_file in sources.source_files.iter() {
            let (header, imports) = read_module_header_and_imports(&source_file.path)?;
            let module_name_span = header.module_name.get_span();
            let module_name = ast::ModuleName::from(header.module_name);

            // Make sure we haven't seen a file with this module name before,
            // otherwise ninja will throw a wobbly
            if let Some(other_file) = module_names_seen.remove(&module_name) {
                let source = std::fs::read_to_string(&source_file.path).into_diagnostic()?;
                let input = NamedSource::new(&source_file.path.to_string_lossy(), source);
                return Err(DuplicateModuleError {
                    input,
                    module_name: module_name.to_string(),
                    module_name_span: (
                        module_name_span.start_offset,
                        module_name_span.end_offset - module_name_span.start_offset,
                    )
                        .into(),
                    other_file: other_file.to_string_lossy().into_owned(),
                }
                .into());
            }
            module_names_seen.insert(module_name.clone(), source_file.path.clone());

            let node = BuildGraphNode {
                package_name: package_name.clone(),
                module_name,
                source_path: source_file.path.to_path_buf(),
                imports,
                document: source_file.document,
            };
            let node_index = build_graph.add_node(node.clone());
            build_graph_nodes.insert(node_index, node);
        }
    }

    // Add the edges
    for (node_index, node) in build_graph_nodes.iter() {
        let node_package_name: Option<&str> =
            node.package_name.as_ref().map(|pkg_name| pkg_name.as_str());

        for import_line in node.imports.iter() {
            let import_package_name: Option<&str> = import_line
                .package
                .as_ref()
                .map(|parens| parens.value.0.value.as_str())
                .or(node_package_name);

            let import_module_name = ast::ModuleName::from(import_line.module_name.clone());

            // Loop through all the nodes and try to
            // find the (import_package_name, import_module_name) we're looking for
            for (
                idx,
                BuildGraphNode {
                    package_name,
                    module_name,
                    ..
                },
            ) in build_graph_nodes.iter()
            {
                let same_package_name = match (package_name, import_package_name) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.as_str() == b,
                    _ => false,
                };
                let same_module_name = *module_name == import_module_name;

                if same_package_name && same_module_name {
                    build_graph.add_edge(*node_index, *idx, "");
                    break;
                }
            }
            // If we can't find the import then we just ignore it,
            // let the checker throw an error.
        }
    }

    check_for_cycles(&build_graph)?;

    Ok((build_graph, build_graph_nodes))
}

fn check_for_cycles(build_graph: &BuildGraph) -> Result<()> {
    let sccs = petgraph::algo::kosaraju_scc(&build_graph);
    for scc in sccs {
        match scc.as_slice() {
            [] => {}
            [node_index] => {
                let is_self_referencing = build_graph.contains_edge(*node_index, *node_index);
                if is_self_referencing {
                    bail!("module `{}` can't import itself!", build_graph[*node_index]);
                    // REVIEW: maybe it would be more helpful to print module _paths_
                    // here rather than module names?
                }
            }
            node_indexes => {
                let mut module_names = node_indexes
                    .iter()
                    .map(|idx| format!("`{}`", build_graph[*idx]))
                    .collect::<Vec<_>>();

                // Sort for determinism
                module_names.sort();

                bail!("modules form a cycle: {}", module_names.join(", "))
            }
        }
    }
    Ok(())
}

/// A representation of the [ninja file syntax](https://github.com/ninja-build/ninja/blob/master/misc/ninja_syntax.py).
#[derive(Debug)]
pub struct BuildNinja {
    variables: HashMap<String, String>,
    rules: Vec<Rule>,
    builds: Vec<Build>,
}

impl BuildNinja {
    fn new(build_dir: &Path, ditto_bin: &Path, compile_subcommand: &'static str) -> Self {
        let build_dir_variable = (
            String::from("builddir"),
            build_dir.to_string_lossy().into_owned(),
        );

        // builddir = $build_dir
        let variables = HashMap::from_iter(vec![(build_dir_variable)]);

        // There will always be at least an `ast` rule rule
        let rules = vec![Rule::new_ast(build_dir, ditto_bin, compile_subcommand)];

        Self {
            variables,
            rules,
            builds: Vec::new(),
        }
    }
    /// Render to `build.ninja` file syntax.
    pub fn into_syntax(self) -> String {
        self.into_syntax_with(|path| path.to_string_lossy().into_owned())
    }

    /// Used for integration testing, where we need predictable path separators.
    pub fn into_syntax_path_slash(self) -> String {
        self.into_syntax_with(|path| path_slash::PathBufExt::to_slash_lossy(&path))
    }

    fn into_syntax_with(self, path_to_string: impl Fn(PathBuf) -> String + Copy) -> String {
        let mut string = String::new();

        if cfg!(debug_assertions) {
            let mut variables = self.variables.into_iter().collect::<Vec<_>>();
            variables.sort();
            for (key, value) in variables {
                string.push_str(&format!("{} = {}", key, value));
                string.push('\n');
                string.push('\n');
            }
        } else {
            for (key, value) in self.variables.into_iter() {
                string.push_str(&format!("{} = {}", key, value));
                string.push('\n');
                string.push('\n');
            }
        };

        let mut rules = self.rules;
        if cfg!(debug_assertions) {
            rules.sort_by(|a, b| a.name.cmp(&b.name));
        }
        for rule in rules {
            string.push_str(&rule.into_syntax());
            string.push('\n');
            string.push('\n');
        }

        let mut builds = self
            .builds
            .into_iter()
            .map(|build| build.into_syntax(path_to_string))
            .collect::<Vec<_>>();

        if cfg!(debug_assertions) {
            builds.sort()
        }
        for build in builds {
            string.push_str(&build);
            string.push('\n');
            string.push('\n');
        }
        string
    }
}

static RULE_NAME_AST: &str = "ast";
static RULE_NAME_JS: &str = "js";
static RULE_NAME_PACKAGE_JSON: &str = "package_json";
static RULE_NAME_DOC_HTML_MODULE: &str = "doc_html_module";
static RULE_NAME_DOC_HTML_INDEX: &str = "doc_html_index";

#[derive(Debug)]
struct Rule {
    name: String,
    command: String,
}

impl Rule {
    fn new_ast(build_dir: &Path, ditto_bin: &Path, compile: &str) -> Self {
        use compile::{ARG_BUILD_DIR, ARG_INPUTS as i, ARG_OUTPUTS as o, SUBCOMMAND_AST as ast};
        let ditto = ditto_bin.to_string_lossy();
        let build_dir = build_dir.to_string_lossy();
        Self {
            name: RULE_NAME_AST.to_string(),
            command: format!(
                "{ditto} {compile} {ast} --{ARG_BUILD_DIR} {build_dir} -{i} ${{in}} -{o} ${{out}}"
            ),
        }
    }

    fn new_js(ditto_bin: &Path, compile: &str) -> Self {
        use compile::{ARG_INPUTS as i, ARG_OUTPUTS as o, SUBCOMMAND_JS as js};
        let ditto = ditto_bin.to_string_lossy();
        Self {
            name: RULE_NAME_JS.to_string(),
            command: format!("{ditto} {compile} {js} -{i} ${{in}} -{o} ${{out}}"),
        }
    }

    fn new_package_json(ditto_bin: &Path, compile: &str) -> Self {
        use compile::{ARG_INPUTS as i, ARG_OUTPUTS as o, SUBCOMMAND_PACKAGE_JSON as package_json};
        let ditto = ditto_bin.to_string_lossy();
        Self {
            name: RULE_NAME_PACKAGE_JSON.to_string(),
            command: format!("{ditto} {compile} {package_json} -{i} ${{in}} -{o} ${{out}}"),
        }
    }

    fn new_doc_html_module(ditto_bin: &Path, compile: &str) -> Self {
        use compile::{
            ARG_INPUTS as i, ARG_OUTPUTS as o, SUBCOMMAND_DOC_HTML_MODULE as doc_html_module,
        };
        let ditto = ditto_bin.to_string_lossy();
        Self {
            name: RULE_NAME_DOC_HTML_MODULE.to_string(),
            command: format!("{ditto} {compile} {doc_html_module} -{i} ${{in}} -{o} ${{out}}"),
        }
    }

    fn new_doc_html_index(ditto_bin: &Path, compile: &str) -> Self {
        use compile::{
            ARG_INPUTS as i, ARG_OUTPUTS as o, SUBCOMMAND_DOC_HTML_INDEX as doc_html_index,
        };
        let ditto = ditto_bin.to_string_lossy();
        Self {
            name: RULE_NAME_DOC_HTML_INDEX.to_string(),
            command: format!("{ditto} {compile} {doc_html_index} -{i} ${{in}} -{o} ${{out}}"),
        }
    }

    fn into_syntax(self) -> String {
        let Self { name, command } = self;
        format!("rule {name}\n  command = {command}")
    }
}

#[derive(Debug)]
struct Build {
    outputs: Vec<PathBuf>,
    rule_name: String,
    inputs: Vec<PathBuf>,
    variables: HashMap<String, String>,
}

impl Build {
    fn new_ast(
        module_descriptor: String,
        ast_path: PathBuf,
        ast_exports_path: PathBuf,
        checker_warnings_path: Option<PathBuf>,
        ditto_source_path: PathBuf,
        dependency_ast_export_paths: Vec<PathBuf>,
    ) -> Self {
        let mut outputs = vec![ast_path, ast_exports_path];
        if let Some(checker_warnings_path) = checker_warnings_path {
            outputs.push(checker_warnings_path);
        }
        let mut inputs = vec![];
        inputs.extend(dependency_ast_export_paths);
        inputs.push(ditto_source_path);

        Self {
            outputs,
            rule_name: String::from(RULE_NAME_AST),
            inputs,
            variables: HashMap::from_iter(vec![(
                String::from("description"),
                format!("Checking {}", module_descriptor),
            )]),
        }
    }

    fn new_js(
        module_descriptor: String,
        js_path: PathBuf,
        //dts_path: PathBuf,
        ast_path: PathBuf,
    ) -> Self {
        let outputs = vec![js_path /*, dts_path */];

        let inputs = vec![ast_path];

        Self {
            outputs,
            rule_name: String::from(RULE_NAME_JS),
            inputs,
            variables: HashMap::from_iter(vec![(
                String::from("description"),
                format!("Generating JavaScript for {}", module_descriptor),
            )]),
        }
    }

    fn new_package_json(
        package_name: &PackageName,
        package_json_path: PathBuf,
        config_path: PathBuf,
    ) -> Self {
        let outputs = vec![package_json_path];

        let inputs = vec![config_path];

        Self {
            outputs,
            rule_name: String::from(RULE_NAME_PACKAGE_JSON),
            inputs,
            variables: HashMap::from_iter(vec![(
                String::from("description"),
                format!("Generating package.json for {}", package_name.as_str()),
            )]),
        }
    }

    fn new_doc_html_module(
        module_descriptor: String,
        html_path: PathBuf,
        ast_exports_path: PathBuf,
    ) -> Self {
        let outputs = vec![html_path];

        let inputs = vec![ast_exports_path];

        Self {
            outputs,
            rule_name: String::from(RULE_NAME_DOC_HTML_MODULE),
            inputs,
            variables: HashMap::from_iter(vec![(
                String::from("description"),
                format!("Generating documentation for {}", module_descriptor),
            )]),
        }
    }

    fn new_doc_html_index(index_html_path: PathBuf, ast_exports_paths: Vec<PathBuf>) -> Self {
        let outputs = vec![index_html_path];

        let inputs = ast_exports_paths;

        Self {
            outputs,
            rule_name: String::from(RULE_NAME_DOC_HTML_INDEX),
            inputs,
            variables: HashMap::from_iter(vec![(
                String::from("description"),
                String::from("Generating documentation index.html"),
            )]),
        }
    }

    fn into_syntax(self, path_to_string: impl Fn(PathBuf) -> String + Copy) -> String {
        // TODO sort for determinism in tests
        let Self { rule_name, .. } = self;

        let mut outputs = self
            .outputs
            .into_iter()
            .map(path_to_string)
            .collect::<Vec<_>>();
        if cfg!(debug_assertions) {
            outputs.sort()
        }
        let outputs = outputs.join(" ");

        let mut inputs = self
            .inputs
            .into_iter()
            .map(path_to_string)
            .collect::<Vec<_>>();
        if cfg!(debug_assertions) {
            inputs.sort()
        }
        let inputs = inputs.join(" ");

        let mut variables = self
            .variables
            .into_iter()
            .map(|(key, value)| format!("\n  {key} = {value}"))
            .collect::<Vec<_>>();

        if cfg!(debug_assertions) {
            variables.sort()
        }
        let variables = variables.join("");

        format!("build {outputs}: {rule_name} {inputs}{variables}",)
    }
}

fn read_module_header_and_imports(path: &Path) -> Result<(cst::Header, Vec<cst::ImportLine>)> {
    let contents = std::fs::read_to_string(path).into_diagnostic()?;
    cst::parse_header_and_imports(&contents)
        .map_err(|err| err.into_report(&path.to_string_lossy(), contents).into())
}
