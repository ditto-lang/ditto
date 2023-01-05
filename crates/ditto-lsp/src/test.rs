use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer};

pub async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) =
        tower_lsp::LspService::new(|client| Backend::new("test".to_string(), client));
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}

#[derive(Debug)]
struct Backend(super::Backend);

impl Backend {
    fn new(version: String, client: Client) -> Self {
        Self(super::Backend::new(version, client))
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        log::info!("< initialize");
        let result = self.0.initialize(params).await;
        if let Ok(ref res) = result {
            log_value("> initialize", res);
        }
        result
    }

    async fn initialized(&self, params: InitializedParams) {
        self.0.initialized(params).await;
        if !self.0.db_documents.is_empty() {
            let mut initial_db_documents = self
                .0
                .db_documents
                .iter()
                .map(|item| (fix_uri(item.key().as_str()), item.value().to_owned()))
                .collect::<Vec<_>>();
            initial_db_documents.sort();
            log::info!("{initial_db_documents:#?}");
        }
        let db = self.0.db.lock().await;
        if !db.documents.is_empty() {
            let mut initial_db_documents = db
                .documents
                .iter()
                .map(|item| (item.key().to_owned(), item.value().to_owned()))
                .collect::<Vec<_>>();
            initial_db_documents.sort();
            log::info!("{initial_db_documents:#?}");
        }
    }

    async fn shutdown(&self) -> Result<()> {
        self.0.shutdown().await
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        log_value("< textDocument/didChange", &params);
        self.0.did_change(params).await
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        log_value("< textDocument/didOpen", &params);
        self.0.did_open(params).await
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        log_value("< textDocument/hover", &params);
        let result = self.0.hover(params).await;
        if let Ok(ref res) = result {
            log_value("> textDocument/hover", res);
        }
        result
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        log_value("< textDocument/definition", &params);
        let result = self.0.goto_definition(params).await;
        if let Ok(ref res) = result {
            log_value("> textDocument/definition", res);
        }
        result
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        log_value("< textDocument/formatting", &params);
        let result = self.0.formatting(params).await;
        if let Ok(ref res) = result {
            log_value("> textDocument/formatting", res);
        }
        result
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        log_value("< textDocument/semanticTokens/full", &params);
        let result = self.0.semantic_tokens_full(params).await;
        if let Ok(ref res) = result {
            log_value("> textDocument/semanticTokens/full", res);
        }
        result
    }
}

fn log_value<T: serde::Serialize>(prefix: &str, value: &T) {
    let json = serde_json::to_value(value).unwrap();
    let json = fix_uri_keys(json);
    let pretty = serde_json::to_string_pretty(&json).unwrap();
    log::info!("{prefix}\n{pretty}");
}

fn fix_uri_keys(json: serde_json::Value) -> serde_json::Value {
    match json {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => json,
        serde_json::Value::Array(array) => {
            serde_json::Value::Array(array.into_iter().map(fix_uri_keys).collect())
        }
        serde_json::Value::Object(object) => serde_json::Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    if let serde_json::Value::String(ref value) = value {
                        if key.to_lowercase().contains("uri") {
                            return (key, serde_json::Value::String(fix_uri(value)));
                        }
                    }
                    (key, fix_uri_keys(value))
                })
                .collect(),
        ),
    }
}

fn fix_uri(value: &str) -> String {
    let uri = tower_lsp::lsp_types::Url::from_file_path(env!("CARGO_WORKSPACE_DIR")).unwrap();
    value.trim_start_matches(&String::from(uri)).to_string()
}
