#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![feature(explicit_generic_args_with_impl_trait)]

mod db;
mod semantic_tokens;

use db::*;
use log::debug;
use miette::IntoDiagnostic;
use serde_json as json;

/// Run the language server.
pub fn main() -> miette::Result<()> {
    // Note that we must have our logging only write out to stderr.
    debug!("starting ditto-lsp");

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = lsp_server::Connection::stdio();

    let capabilities = init_capabilities();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = json::to_value(&capabilities).into_diagnostic()?;

    let _initialization_params = connection
        .initialize(server_capabilities)
        .into_diagnostic()?;

    // REVIEW: do we need to `cd` into the rootDir that comes with the initialization_params?

    main_loop(connection)?;

    io_threads.join().into_diagnostic()?;

    Ok(())
}

fn init_capabilities() -> lsp_types::ServerCapabilities {
    use lsp_types::*;

    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL, // TODO INCREMENTAL?
        )),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                full: Some(SemanticTokensFullOptions::Bool(true)),
                legend: semantic_tokens::legend(),
                range: Some(false),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(false),
                },
            },
        )),
        document_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    }
}

fn main_loop(connection: lsp_server::Connection) -> miette::Result<()> {
    debug!("starting ditto-lsp main loop");

    let mut db = Database::new();

    for msg in &connection.receiver {
        use lsp_server::Message::*;
        match msg {
            Request(req) => {
                if connection.handle_shutdown(&req).into_diagnostic()? {
                    return Ok(());
                }

                use lsp_types::request::{Formatting, SemanticTokensFullRequest};
                match cast_request::<SemanticTokensFullRequest>(req) {
                    Ok(request) => handle_semantic_tokens_request(&db, &connection, request)?,
                    Err(req) => match cast_request::<Formatting>(req) {
                        Ok(request) => handle_formatting_request(&db, &connection, request)?,

                        // Unsupported method
                        Err(request) => connection
                            .sender
                            .send(lsp_server::Message::Response(lsp_server::Response {
                                id: request.id,
                                result: None,
                                error: Some(lsp_server::ResponseError {
                                    message: format!("{} not supported", request.method),
                                    code: lsp_server::ErrorCode::MethodNotFound as i32,
                                    data: None,
                                }),
                            }))
                            .into_diagnostic()?,
                    },
                };
            }
            Response(_response) => {
                // ignored
            }
            Notification(notification) => {
                use lsp_types::notification::{DidChangeTextDocument, DidOpenTextDocument};
                match cast_notification::<DidOpenTextDocument>(notification) {
                    Ok(params) => {
                        db.set_source(params.text_document.uri, params.text_document.text);
                    }
                    Err(notification) => {
                        match cast_notification::<DidChangeTextDocument>(notification) {
                            Ok(params) => {
                                for change in params.content_changes {
                                    db.set_source(
                                        params.text_document.uri.clone(),
                                        change.text, // because TextDocumentSyncKind::FULL
                                    );
                                }
                            }
                            Err(_notification) => {
                                // TODO: handle creates, renames, and deletes
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_formatting_request(
    db: &Database,
    connection: &lsp_server::Connection,
    request: (lsp_server::RequestId, lsp_types::DocumentFormattingParams),
) -> miette::Result<()> {
    handle_request::<lsp_types::request::Formatting>(connection, request, |params| {
        let source = db.source(params.text_document.uri);
        match ditto_cst::Module::parse(&source) {
            Ok(module) => {
                let formatted = ditto_fmt::format_module(module);
                let edit = lsp_types::TextEdit {
                    range: lsp_types::Range {
                        start: lsp_types::Position {
                            line: 0,
                            character: 0,
                        },
                        end: lsp_types::Position {
                            line: source.lines().count() as u32,
                            character: source.lines().last().map_or(0, |line| line.len() as u32),
                        },
                    },
                    new_text: formatted,
                };
                Ok(Some(vec![edit]))
            }
            Err(_parse_error) => {
                // NOTE: responding with the error like this is
                // actually just annoying...(at least in vscode)
                //
                //Err(lsp_server::ResponseError {
                //    code: lsp_server::ErrorCode::ParseError as i32,
                //    message: format!("{:?}", parse_error),
                //    data: None,
                //}),
                Ok(None)
            }
        }
    })
}

fn handle_semantic_tokens_request(
    db: &Database,
    connection: &lsp_server::Connection,
    request: (lsp_server::RequestId, lsp_types::SemanticTokensParams),
) -> miette::Result<()> {
    handle_request::<lsp_types::request::SemanticTokensFullRequest>(connection, request, |params| {
        let source = db.source(params.text_document.uri);

        // NOTE: we can't cache trees in the salsa db
        // as tree_sitter::Tree doesn't implement Eq :(
        //
        // Although we could use a newtype and define Eq using the S-expression,
        // but I don't think we gain that much from having caching here anyway?
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(tree_sitter_ditto::language())
            .map_err(|err| lsp_server::ResponseError {
                code: lsp_server::ErrorCode::InternalError as i32,
                message: format!("error initialising tree-sitter parser: {}", err),
                data: None,
            })?;
        let tree = parser
            .parse(&source, None)
            .ok_or(lsp_server::ResponseError {
                code: lsp_server::ErrorCode::InternalError as i32,
                message: "unable to parse source tree".to_string(),
                data: None,
            })?;

        let tokens = semantic_tokens::get_tokens(&tree, &source);
        Ok(Some(lsp_types::SemanticTokensResult::Tokens(tokens)))
    })
}

fn handle_request<R>(
    connection: &lsp_server::Connection,
    (request_id, params): (lsp_server::RequestId, R::Params),
    handler: impl FnOnce(R::Params) -> Result<R::Result, lsp_server::ResponseError>,
) -> miette::Result<()>
where
    R: lsp_types::request::Request,
{
    let response = handler(params);
    respond::<R>(response, request_id, connection)
}

fn respond<R>(
    response: Result<R::Result, lsp_server::ResponseError>,
    request_id: lsp_server::RequestId,
    connection: &lsp_server::Connection,
) -> miette::Result<()>
where
    R: lsp_types::request::Request,
{
    match response {
        Ok(result) => connection
            .sender
            .send(lsp_server::Message::Response(lsp_server::Response {
                id: request_id,
                result: Some(json::to_value(&result).unwrap()),
                error: None,
            }))
            .into_diagnostic(),
        Err(error) => connection
            .sender
            .send(lsp_server::Message::Response(lsp_server::Response {
                id: request_id,
                result: None,
                error: Some(error),
            }))
            .into_diagnostic(),
    }
}

fn cast_notification<N>(
    not: lsp_server::Notification,
) -> Result<N::Params, lsp_server::Notification>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    not.extract(N::METHOD)
}

fn cast_request<R>(
    req: lsp_server::Request,
) -> Result<(lsp_server::RequestId, R::Params), lsp_server::Request>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

#[allow(dead_code)]
fn lsp_log_info(message: String, connection: &lsp_server::Connection) {
    notify::<lsp_types::notification::LogMessage>(
        lsp_types::LogMessageParams {
            typ: lsp_types::MessageType::INFO,
            message,
        },
        connection,
    )
    .unwrap();
}

#[allow(dead_code)]
fn notify<N>(params: N::Params, connection: &lsp_server::Connection) -> miette::Result<()>
where
    N: lsp_types::notification::Notification,
{
    connection
        .sender
        .send(lsp_server::Message::Notification(
            lsp_server::Notification {
                method: N::METHOD.to_string(),
                params: serde_json::to_value(params).unwrap(),
            },
        ))
        .into_diagnostic()
}
