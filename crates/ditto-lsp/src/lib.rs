#![feature(box_patterns)]

mod common;
mod db;
mod goto_definition;
mod hover;
mod locate;
mod semantic_tokens;
mod test;

pub use test::main as main_test;

use common::{offset_to_position, position_to_offset};
use ditto_ast::{self as ast, FullyQualifiedModuleName};
use ditto_cst as cst;
use ditto_make::find_ditto_files;
use ropey::Rope;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tower_lsp::{jsonrpc, lsp_types::*, Client, LanguageServer};

static SERVER_NAME: &str = "ditto";

pub async fn main(version: String) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = tower_lsp::LspService::new(|client| Server::new(version, client));
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}

#[derive(Debug)]
struct Server {
    client: Client,
    backend: Arc<Mutex<Backend>>,
}

impl Server {
    fn new(version: String, client: Client) -> Self {
        Self {
            client,
            backend: Arc::new(Mutex::new(Backend::new(version))),
        }
    }
}

#[derive(Debug)]
struct Backend {
    version: String,
    db: db::Database,
    documents: Documents,
    highlight_query: Option<ditto_highlight::Query>,
    project_config: Option<ProjectConfig>,
}

type Documents = HashMap<Url, (Option<FullyQualifiedModuleName>, db::Document)>;

type Diagnostics = Vec<db::Diagnostic>;

#[derive(Debug)]
struct ProjectConfig {
    /// Canonical path to .ditto/packages
    packages_dir: PathBuf,
}

impl ProjectConfig {
    fn initialize(root: Url) -> Option<Self> {
        let mut config = root.to_file_path().ok()?;
        config.push(ditto_config::CONFIG_FILE_NAME);
        if config.exists() {
            let config = ditto_config::read_config(&config).ok()?;
            let mut packages_dir = config.ditto_dir;
            packages_dir.push("packages"); // HACK
            return Some(ProjectConfig { packages_dir });
        }
        None
    }

    fn package_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![];
        if let Ok(entries) = std::fs::read_dir(&self.packages_dir) {
            for entry in entries {
                if let Ok(path) = entry.map(|entry| entry.path()) {
                    if path.is_dir() {
                        dirs.push(path)
                    }
                }
            }
        }
        dirs
    }

    fn uri_to_package_name(&self, uri: &Url) -> Result<Option<ast::PackageName>, ()> {
        if !self.packages_dir.exists() {
            // packages directory doesn't exist,
            // so assume this uri belongs to the current package
            return Ok(None);
        }

        let packages_dir = self.packages_dir.canonicalize().map_err(|_| ())?;
        let uri = uri.to_file_path()?;
        if !uri.starts_with(&packages_dir) {
            // uri isn't beneath the packages directory
            return Ok(None);
        }
        let uri = uri.strip_prefix(packages_dir).map_err(|_| ())?;
        if let Some(package_name) = uri.components().next().map(package_name_from_component) {
            // yup, this is a package
            Ok(Some(package_name))
        } else {
            // error extracting package name
            Err(())
        }
    }
}

impl Backend {
    fn new(version: String) -> Self {
        Self {
            version,
            db: db::Database::default(),
            documents: Documents::new(),
            highlight_query: ditto_highlight::try_init_query().ok(),
            project_config: None,
        }
    }

    fn project_server_capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            ..self.non_project_server_capabilities()
        }
    }

    fn non_project_server_capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),

            document_formatting_provider: Some(OneOf::Left(true)),
            semantic_tokens_provider: self.highlight_query.as_ref().map(|_| {
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                    range: Some(false),
                    legend: semantic_tokens::legend(),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                })
            }),
            ..Default::default()
        }
    }

    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: SERVER_NAME.to_string(),
            version: Some(self.version.clone()),
        }
    }

    fn insert_document(&mut self, uri: Url, version: i32, text: String) {
        let version = Some(version);
        let fully_qualified_module_name = self.get_fully_qualified_module_name(&uri, &text);
        let rope = Rope::from(text);
        let document = db::Document::new(&self.db, version, uri.clone(), rope);
        if let Some(ref mn) = fully_qualified_module_name {
            self.db.insert_document(mn.clone(), document);
        }
        self.documents
            .insert(uri, (fully_qualified_module_name, document));
    }

    fn update_document(
        &mut self,
        uri: Url,
        version: i32,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    ) {
        if let Some((_, document)) = self.documents.remove(&uri) {
            let rope = document.rope(&self.db);
            let fully_qualified_module_name =
                self.get_fully_qualified_module_name(&uri, &rope.to_string());

            if !content_changes.is_empty() {
                let mut rope = rope.clone();
                if apply_content_changes_to_rope(&mut rope, content_changes).is_err() {
                    // Something went horribly wrong, forget about this document!
                    return;
                }
                document.set_rope(&mut self.db).to(rope);
            }

            if *document.uri(&self.db) != uri {
                document.set_uri(&mut self.db).to(uri.clone());
            }
            if *document.version(&self.db) != Some(version) {
                document.set_version(&mut self.db).to(Some(version));
            }

            if let Some(ref mn) = fully_qualified_module_name {
                self.db.update_document(mn.clone(), document);
            }

            self.documents
                .insert(uri, (fully_qualified_module_name, document));
        }

        // TODO: what should we do if we don't know about this uri?
    }

    fn get_fully_qualified_module_name(
        &self,
        uri: &Url,
        source: &str,
    ) -> Option<ast::FullyQualifiedModuleName> {
        let cst::Header { module_name, .. } = cst::partial_parse_header(source).ok()?;
        let module_name = ast::ModuleName::from(module_name);
        if let Some(ref project_config) = self.project_config {
            if let Ok(package_name) = project_config.uri_to_package_name(uri) {
                Some((package_name, module_name))
            } else {
                // shrug ?
                None
            }
        } else {
            Some((None, module_name))
        }
    }

    fn check_module(&self, uri: &Url) -> (Option<ast::Module>, Diagnostics) {
        if let Some((_, document)) = self.documents.get(uri) {
            if let Some(ref project_config) = self.project_config {
                if let Ok(package_name) = project_config.uri_to_package_name(uri) {
                    let module = db::parse_and_check(&self.db, *document, package_name.clone());
                    let diagnostics = db::parse_and_check::accumulated::<db::Diagnostics>(
                        &self.db,
                        *document,
                        package_name,
                    );
                    return (module, diagnostics);
                }
            } else {
                // TODO: report parse errors, at least?
            }
        }
        (None, vec![])
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Server {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        let mut backend = self.backend.clone().lock_owned().await;
        if let Some(project_config) = params.root_uri.and_then(ProjectConfig::initialize) {
            backend.project_config = Some(project_config);

            Ok(InitializeResult {
                server_info: Some(backend.server_info()),
                capabilities: backend.project_server_capabilities(),
            })
        } else {
            Ok(InitializeResult {
                server_info: Some(backend.server_info()),
                capabilities: backend.non_project_server_capabilities(),
            })
        }
    }

    async fn initialized(&self, _: InitializedParams) {
        let mut backend = self.backend.clone().lock_owned().await;
        if let Some(ref project_config) = backend.project_config {
            let mut documents = read_package_files(&backend.db, ".".into(), None);
            for dir in project_config.package_dirs() {
                if let Some(package_name) = dir.components().last().map(package_name_from_component)
                {
                    documents.extend(read_package_files(&backend.db, dir, Some(package_name)));
                }
            }
            for (_, (key, document)) in documents.iter() {
                if let Some(key) = key {
                    backend.db.insert_document(key.clone(), *document);
                }
            }
            backend.documents = documents;
        }
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri, version, text, ..
            },
        } = params;
        let (_, diagnostics) = {
            let mut backend = self.backend.clone().lock_owned().await;
            backend.insert_document(uri.clone(), version, text);
            backend.check_module(&uri)
        };
        publish_diagnostics(&self.client, diagnostics).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes,
        } = params;
        let (_, diagnostics) = {
            let mut backend = self.backend.clone().lock_owned().await;
            backend.update_document(uri.clone(), version, content_changes);
            backend.check_module(&uri)
        };
        publish_diagnostics(&self.client, diagnostics).await;
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let HoverParams {
            text_document_position_params:
                TextDocumentPositionParams {
                    text_document,
                    position,
                },
            ..
        } = params;
        let backend = self.backend.clone().lock_owned().await;
        Ok((|| {
            let (_, document) = backend.documents.get(&text_document.uri)?;
            let module = backend.check_module(&text_document.uri).0?;
            let rope = document.rope(&backend.db);
            let source = rope.to_string();
            let offset = position_to_offset(position, rope)?;
            let located = locate::locate(&source, module, offset)?;
            hover::hover(located, rope)
        })())
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let backend = self.backend.clone().lock_owned().await;
        Ok((|| {
            let (_, document) = backend.documents.get(&params.text_document.uri)?;
            let rope = document.rope(&backend.db);
            let cst_module = cst::Module::parse(&rope.to_string()).ok()?;
            let edits = fmt(cst_module, rope);
            Some(edits)
        })())
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let GotoDefinitionParams {
            text_document_position_params:
                TextDocumentPositionParams {
                    text_document,
                    position,
                },
            ..
        } = params;
        let backend = self.backend.clone().lock_owned().await;
        Ok((|| {
            let (_, document) = backend.documents.get(&text_document.uri)?;
            let module = backend.check_module(&text_document.uri).0?;
            let rope = document.rope(&backend.db);
            let source = rope.to_string();
            let offset = position_to_offset(position, rope)?;
            let located = locate::locate(&source, module, offset)?;
            goto_definition::goto_definition(&backend.db, located)
        })())
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let backend = self.backend.clone().lock_owned().await;
        Ok((|| {
            let query = backend.highlight_query.as_ref()?;
            let (_, document) = backend.documents.get(&params.text_document.uri)?;
            let mut parser = ditto_tree_sitter::try_init_parser().ok()?;
            let rope = document.rope(&backend.db);
            let source = rope.to_string();
            let tree = parser.parse(&source, None)?;
            let tokens = semantic_tokens::get_tokens(&tree, &source, query);
            Some(SemanticTokensResult::Tokens(tokens))
        })())
    }
}

async fn publish_diagnostics(client: &Client, diagnostics: Vec<db::Diagnostic>) {
    let mut keyed_diagnostics: HashMap<(Url, db::DocumentVersion), Vec<Diagnostic>> =
        HashMap::new();

    for db::Diagnostic {
        diagnostic,
        uri,
        version,
    } in diagnostics
    {
        let key = (uri, version);
        if let Some(entry) = keyed_diagnostics.get_mut(&key) {
            if let Some(diagnostic) = diagnostic {
                entry.push(diagnostic);
            }
        } else {
            keyed_diagnostics.insert(key, diagnostic.into_iter().collect());
        }
    }
    for ((url, version), diagnostics) in keyed_diagnostics {
        if cfg!(debug_assertions) && diagnostics.is_empty() {
            // Need to do this for the lsp-test suite :(
            continue;
        }
        client.publish_diagnostics(url, diagnostics, version).await;
    }
}

fn package_name_from_component(component: std::path::Component) -> ast::PackageName {
    ast::PackageName(component.as_os_str().to_string_lossy().into_owned())
}

fn apply_content_changes_to_rope(
    rope: &mut Rope,
    content_changes: Vec<TextDocumentContentChangeEvent>,
) -> ropey::Result<()> {
    for change in content_changes {
        if let Some(start) = change
            .range
            .and_then(|range| position_to_offset(range.start, rope))
            .and_then(|start| rope.try_byte_to_char(start).ok())
        {
            if !change.text.is_empty() {
                rope.try_insert(start, &change.text)?;
            } else if let Some(end) = change
                .range
                .and_then(|range| position_to_offset(range.end, rope))
                .and_then(|end| rope.try_byte_to_char(end).ok())
            {
                rope.try_remove(start..end)?;
            }
        }
    }
    Ok(())
}

fn read_package_files(
    db: &db::Database,
    dir: PathBuf,
    package_name: Option<ast::PackageName>,
) -> Documents {
    let mut documents = Documents::new();

    let mut config_file = dir.clone();
    config_file.push(ditto_config::CONFIG_FILE_NAME);

    if let Ok(config) = ditto_config::read_config(&config_file) {
        let mut src_dir = dir.clone();
        src_dir.push(config.src_dir);

        let mut test_dir = dir;
        test_dir.push(config.test_dir);

        let ditto_files = find_ditto_files(src_dir)
            .unwrap_or_default()
            .into_iter()
            .chain(find_ditto_files(test_dir).unwrap_or_default().into_iter());

        for ditto_file in ditto_files {
            if let Some((uri, rope)) = read_file(ditto_file) {
                let module_name = cst::partial_parse_header(&rope.to_string())
                    .ok()
                    .map(|header| (package_name.clone(), header.module_name.into()));
                let document = db::Document::new(db, None, uri.clone(), rope);
                documents.insert(uri, (module_name, document));
            }
        }
    }
    documents
}

fn read_file(path: std::path::PathBuf) -> Option<(Url, Rope)> {
    let file = std::fs::File::open(&path).ok()?;
    let reader = std::io::BufReader::new(file);
    let rope = Rope::from_reader(reader).ok()?;
    let ditto_file = std::fs::canonicalize(path).ok()?;
    let uri = Url::from_file_path(ditto_file).ok()?;
    Some((uri, rope))
}

fn fmt(cst_module: cst::Module, rope: &Rope) -> Vec<TextEdit> {
    let indexed_text = lsp_document::IndexedText::new(rope.to_string());
    let before = rope.bytes().collect::<Vec<_>>();
    let edits = ditto_fmt::format_module_edits(cst_module, &before);
    edits
        .into_iter()
        .filter_map(
            |ditto_fmt::Edit {
                 from,
                 to,
                 replacement,
             }| {
                let new_text = std::str::from_utf8(&replacement).ok()?.to_owned();
                let start = offset_to_position(from, &indexed_text)?;
                let end = offset_to_position(to, &indexed_text)?;
                let range = Range { start, end };
                Some(TextEdit { range, new_text })
            },
        )
        .collect()
}
