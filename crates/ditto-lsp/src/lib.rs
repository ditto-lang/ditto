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
use dashmap::DashMap;
use ditto_ast as ast;
use ditto_cst as cst;
use ditto_make::find_ditto_files;
use ropey::Rope;
use std::path::PathBuf;
use tokio::sync::Mutex;
use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer};

pub async fn main(version: String) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = tower_lsp::LspService::new(|client| Backend::new(version, client));
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}

#[derive(Debug)]
struct Backend {
    version: String,
    client: Client,
    db: Mutex<db::Database>,
    db_documents: DashMap<Url, db::Document>,
    unparsed: DashMap<Url, (Rope, db::DocumentVersion)>,
    highlight_query: Option<ditto_highlight::Query>,
    project_config: Mutex<Option<ProjectConfig>>,
}

#[derive(Debug)]
struct ProjectConfig {
    packages_dir: PathBuf,
}

impl Backend {
    fn new(version: String, client: Client) -> Self {
        Self {
            version,
            client,
            db: Mutex::new(db::Database::default()),
            db_documents: DashMap::new(),
            unparsed: DashMap::new(),
            highlight_query: ditto_highlight::try_init_query().ok(),
            project_config: Mutex::new(None),
        }
    }

    async fn get_module(&self, uri: &Url) -> Option<ast::Module> {
        if let Ok(package_name) = self.uri_to_package_name(uri).await {
            let db = self.db.lock().await;
            if let Some(document) = self.db_documents.get(uri) {
                let module = db::parse_and_check(&*db, *document, package_name.clone());
                let diagnostics = db::parse_and_check::accumulated::<db::Diagnostics>(
                    &*db,
                    *document,
                    package_name,
                );
                self.publish_diagnostics(diagnostics).await;
                return module;
            }
        }
        None
    }

    async fn uri_to_package_name(
        &self,
        uri: &Url,
    ) -> std::result::Result<Option<ast::PackageName>, ()> {
        if let Some(ProjectConfig { ref packages_dir }) = *self.project_config.lock().await {
            if packages_dir.exists() {
                let packages_dir = packages_dir.canonicalize().map_err(|_| ())?;
                let uri = uri.to_file_path()?;
                if uri.starts_with(&packages_dir) {
                    let uri = uri.strip_prefix(packages_dir).map_err(|_| ())?;
                    if let Some(package_name) =
                        uri.components().next().map(package_name_from_component)
                    {
                        // yup, this is a package
                        return Ok(Some(package_name));
                    } else {
                        // error extracting package name
                        return Err(());
                    }
                } else {
                    // uri isn't beneath the packages directory
                    return Ok(None);
                }
            } else {
                // packages directory doesn't exist,
                // so assume this uri belongs to the current package
                return Ok(None);
            }
        }
        // not a project!?
        Err(())
    }

    async fn publish_document_diagnostics(&self, uri: &Url) {
        self.get_module(uri).await;
    }

    async fn publish_diagnostics(&self, diagnostics: Vec<db::Diagnostic>) {
        use std::collections::HashMap;

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
            self.client
                .publish_diagnostics(url, diagnostics, version)
                .await;
        }
    }

    async fn hover_impl(&self, params: HoverParams) -> Option<Hover> {
        let HoverParams {
            text_document_position_params:
                TextDocumentPositionParams {
                    text_document,
                    position,
                },
            ..
        } = params;

        let module = self.get_module(&text_document.uri).await?;
        let document = self.db_documents.get(&text_document.uri)?;
        let db = self.db.lock().await;
        let rope = document.rope(&*db);
        let source = rope.to_string();
        let offset = position_to_offset(position, rope)?;
        let located = locate::locate(&source, module, offset)?;
        hover::hover(located, rope)
    }

    async fn goto_definition_impl(
        &self,
        params: GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        let GotoDefinitionParams {
            text_document_position_params:
                TextDocumentPositionParams {
                    text_document,
                    position,
                },
            ..
        } = params;

        let module = self.get_module(&text_document.uri).await?;
        let document = self.db_documents.get(&text_document.uri)?;
        let db = self.db.lock().await;
        let rope = document.rope(&*db);
        let source = rope.to_string();
        let offset = position_to_offset(position, rope)?;
        let located = locate::locate(&source, module, offset)?;
        goto_definition::goto_definition(&db, located)
    }

    async fn insert_document(
        &self,
        uri: Url,
        rope: Rope,
        version: db::DocumentVersion,
        package: Option<ast::PackageName>,
    ) -> bool {
        match cst::partial_parse_header(&rope.to_string()) {
            Ok(header) => {
                self.unparsed.remove(&uri);

                let db = self.db.lock().await;
                let document = db::Document::new(&*db, version, uri.clone(), rope);
                self.db_documents.insert(uri, document);
                db.documents
                    .insert((package, header.module_name.into()), document);
                // report synthetic write?
                true
            }
            Err(err) => {
                if let Some(diagnostic) = common::parse_error_into_lsp_diagnostic(err, &uri, &rope)
                {
                    self.client
                        .publish_diagnostics(uri.clone(), vec![diagnostic], version)
                        .await;
                }
                self.unparsed.insert(uri, (rope, version));
                false
            }
        }
    }

    async fn initialize_db_documents(&self) {
        self.read_package_files(".".into(), None).await;
        let package_dirs = self.package_dirs().await;
        for dir in package_dirs {
            if let Some(package_name) = dir.components().last().map(package_name_from_component) {
                self.read_package_files(dir, Some(package_name)).await;
            }
        }
    }

    async fn is_project(&self) -> bool {
        self.project_config.lock().await.is_some()
    }

    async fn package_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = vec![];
        if let Some(ref project_config) = *self.project_config.lock().await {
            if let Ok(entries) = std::fs::read_dir(&project_config.packages_dir) {
                for entry in entries {
                    if let Ok(path) = entry.map(|entry| entry.path()) {
                        if path.is_dir() {
                            dirs.push(path)
                        }
                    }
                }
            }
        }
        dirs
    }

    async fn read_package_files(&self, dir: PathBuf, package: Option<ast::PackageName>) {
        let mut config_file = dir.clone();
        config_file.push(ditto_config::CONFIG_FILE_NAME);
        if let Ok(config) = ditto_config::read_config(&config_file) {
            let mut src_dir = dir.clone();
            src_dir.push(config.src_dir);

            let mut test_dir = dir;
            test_dir.push(config.test_dir);

            let sources = find_ditto_files(src_dir)
                .unwrap_or_default()
                .into_iter()
                .chain(find_ditto_files(test_dir).unwrap_or_default().into_iter());

            for ditto_file in sources {
                if let Some((uri, rope)) = file_to_document(ditto_file) {
                    if !self.db_documents.contains_key(&uri) {
                        self.insert_document(uri, rope, None, package.clone()).await;
                    }
                }
            }
        }
    }

    async fn open_document(&self, params: DidOpenTextDocumentParams) {
        let DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri, version, text, ..
            },
        } = params;

        // Do we know about this document already?
        if let Some(document) = self.db_documents.get(&uri) {
            let mut db = self.db.lock().await;
            // Do we need to update the version number?
            // (if it was added by `initialise_db_documents` then the version will be None)
            let document_version = document.version(&*db);
            if let Some(document_version) = document_version {
                if *document_version != version {
                    document.set_version(&mut *db).to(Some(version));
                }
            } else {
                document.set_version(&mut *db).to(Some(version));
            }
        } else {
            // If for some reason(?) we don't already know about it, then insert it
            self.insert_document(uri.clone(), Rope::from(text), Some(version), None)
                .await;
        }
        self.publish_document_diagnostics(&uri).await;
    }

    async fn update_document(&self, params: DidChangeTextDocumentParams) {
        let DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { ref uri, version },
            ref content_changes,
        } = params;

        if let Some(document) = self.db_documents.get(uri) {
            {
                let mut db = self.db.lock().await;
                let mut rope = document.rope(&*db).clone(); // FIXME: would be nice not to have to clone here?
                apply_changes(&mut rope, content_changes);
                document.set_rope(&mut *db).to(rope);
                document.set_version(&mut *db).to(Some(version));

                // TODO: check if module name has changed
            }
            self.publish_document_diagnostics(uri).await;
        } else if let Some((_, (mut rope, _))) = self.unparsed.remove(uri) {
            apply_changes(&mut rope, content_changes);
            let inserted = self
                .insert_document(uri.clone(), rope, Some(version), None)
                .await;
            if inserted {
                // try again
                self.did_change(params).await;
            }
        }
    }

    async fn format_document(&self, uri: &Url) -> Option<Vec<TextEdit>> {
        let document = self.db_documents.get(uri)?;
        let db = self.db.lock().await;
        let rope = document.rope(&*db);
        if let Ok(cst_module) = cst::Module::parse(&rope.to_string()) {
            let edits = fmt(cst_module, rope);
            return Some(edits);
        }
        None
    }
    async fn semantic_tokens_full_impl(
        &self,
        params: SemanticTokensParams,
    ) -> Option<SemanticTokensResult> {
        let query = self.highlight_query.as_ref()?;
        let document = self.db_documents.get(&params.text_document.uri)?;
        let mut parser = ditto_tree_sitter::try_init_parser().ok()?;
        let db = self.db.lock().await;
        let rope = document.rope(&*db);
        let source = rope.to_string();
        let tree = parser.parse(&source, None)?;
        let tokens = semantic_tokens::get_tokens(&tree, &source, query);
        Some(SemanticTokensResult::Tokens(tokens))
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
            name: "ditto".to_string(),
            version: Some(self.version.clone()),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root) = params.root_uri {
            if let Ok(mut config) = root.to_file_path() {
                config.push(ditto_config::CONFIG_FILE_NAME);
                if config.exists() {
                    if let Ok(config) = ditto_config::read_config(&config) {
                        let mut packages_dir = config.ditto_dir;
                        packages_dir.push("packages"); // HACK

                        let project_config = ProjectConfig { packages_dir };
                        *self.project_config.lock().await = Some(project_config);
                        return Ok(InitializeResult {
                            server_info: Some(self.server_info()),
                            capabilities: self.project_server_capabilities(),
                        });
                    }
                }
            }
        }

        Ok(InitializeResult {
            server_info: Some(self.server_info()),
            capabilities: self.non_project_server_capabilities(),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        if self.is_project().await {
            self.initialize_db_documents().await;
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.update_document(params).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.open_document(params).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let result = self.hover_impl(params).await;
        Ok(result)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let result = self.goto_definition_impl(params).await;
        Ok(result)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let result = self.format_document(&params.text_document.uri).await;
        Ok(result)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let result = self.semantic_tokens_full_impl(params).await;
        Ok(result)
    }
}

fn fmt(cst_module: cst::Module, rope: &Rope) -> Vec<TextEdit> {
    let before = rope.bytes().collect::<Vec<_>>();
    let after = ditto_fmt::format_module(cst_module).as_bytes().to_owned();
    let diffs = similar::capture_diff_slices(similar::Algorithm::Myers, &before, &after);
    diffs
        .into_iter()
        .filter_map(|diff| match diff {
            similar::DiffOp::Equal { .. } => None,
            similar::DiffOp::Delete {
                old_index, old_len, ..
            } => {
                let start = offset_to_position(old_index, rope)?;
                let end = offset_to_position(old_index + old_len, rope)?;
                let range = Range { start, end };
                let new_text = "".to_string();
                let edit = TextEdit { range, new_text };
                Some(edit)
            }
            similar::DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => {
                let start = offset_to_position(old_index, rope)?;
                let end = start;
                let range = Range { start, end };
                let new_text = std::str::from_utf8(&after[new_index..new_index + new_len])
                    .ok()?
                    .to_string();
                let edit = TextEdit { range, new_text };
                Some(edit)
            }
            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let start = offset_to_position(old_index, rope)?;
                let end = offset_to_position(old_index + old_len, rope)?;
                let range = Range { start, end };
                let new_text = std::str::from_utf8(&after[new_index..new_index + new_len])
                    .ok()?
                    .to_string();
                let edit = TextEdit { range, new_text };
                Some(edit)
            }
        })
        .collect()
}

fn apply_changes(rope: &mut Rope, changes: &[TextDocumentContentChangeEvent]) {
    for change in changes {
        if let Some(start) = change
            .range
            .and_then(|range| position_to_offset(range.start, rope))
        {
            if !change.text.is_empty() {
                rope.insert(start, &change.text);
            } else if let Some(end) = change
                .range
                .and_then(|range| position_to_offset(range.end, rope))
            {
                rope.remove(start..end);
            }
        }
    }
}

fn file_to_document(path: std::path::PathBuf) -> Option<(Url, Rope)> {
    let file = std::fs::File::open(&path).ok()?;
    let rope = Rope::from_reader(file).ok()?;
    let ditto_file = std::fs::canonicalize(path).ok()?;
    let uri = Url::from_file_path(ditto_file).ok()?;
    Some((uri, rope))
}

fn package_name_from_component(component: std::path::Component) -> ast::PackageName {
    ast::PackageName(component.as_os_str().to_string_lossy().into_owned())
}
