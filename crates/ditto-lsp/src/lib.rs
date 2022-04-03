#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod semantic_tokens;

use log::debug;
use miette::IntoDiagnostic;
use serde_json as json;
use std::collections::HashMap;
use url::Url;

/// Run the language server.
pub fn main() -> miette::Result<()> {
    // Note that we must have our logging only write out to stderr.
    debug!("starting ditto-lsp");

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = lsp_server::Connection::stdio();

    let capabilities = lsp_types::ServerCapabilities {
        text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Kind(
            lsp_types::TextDocumentSyncKind::FULL, // TODO INCREMENTAL
        )),
        semantic_tokens_provider: Some(
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                lsp_types::SemanticTokensOptions {
                    full: Some(lsp_types::SemanticTokensFullOptions::Bool(true)),
                    legend: semantic_tokens::legend(),
                    range: Some(false),
                    work_done_progress_options: lsp_types::WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                },
            ),
        ),
        document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        //definition_provider: Some(lsp_types::OneOf::Left(true)),
        ..Default::default()
    };

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = json::to_value(&capabilities).unwrap();

    let initialization_params = connection
        .initialize(server_capabilities)
        .into_diagnostic()?;

    main_loop(connection, initialization_params)?;

    io_threads.join().into_diagnostic()?;

    // Shut down gracefully.
    debug!("shutting down ditto-lsp");
    Ok(())
}

fn main_loop(connection: lsp_server::Connection, params: json::Value) -> miette::Result<()> {
    debug!("starting ditto-lsp main loop");

    let _params: lsp_types::InitializeParams = json::from_value(params).unwrap();

    let mut trees = Trees::new();

    'msg_loop: for msg in &connection.receiver {
        match msg {
            lsp_server::Message::Request(req) => {
                if connection.handle_shutdown(&req).into_diagnostic()? {
                    return Ok(());
                }
                use lsp_types::request::{Formatting, SemanticTokensFullRequest};

                // TODO break out some `handle` function to enforce that requests
                // are always responded to (correctly).
                match cast_request::<SemanticTokensFullRequest>(req) {
                    Ok((request_id, params)) => {
                        if let Some((tree, source)) = trees.get(&params.text_document.uri) {
                            let tokens = semantic_tokens::get_tokens(tree, source);
                            respond::<SemanticTokensFullRequest>(
                                Ok(Some(lsp_types::SemanticTokensResult::Tokens(tokens))),
                                request_id,
                                &connection,
                            )?;
                        } else {
                            respond::<SemanticTokensFullRequest>(
                                Err(lsp_server::ResponseError {
                                    code: lsp_server::ErrorCode::InternalError as i32,
                                    message: format!("no tree for {}", params.text_document.uri),
                                    data: None,
                                }),
                                request_id,
                                &connection,
                            )?;
                        }
                        continue 'msg_loop;
                    }
                    // TODO can we fix this matching pattern to avoid the code
                    // marching off the screen?
                    Err(req) => match cast_request::<Formatting>(req) {
                        Ok((request_id, params)) => {
                            if let Some((_, contents)) = trees.get(&params.text_document.uri) {
                                match ditto_cst::Module::parse(contents) {
                                    Ok(module) => {
                                        let formatted = ditto_fmt::format_module(module);
                                        let edit = lsp_types::TextEdit {
                                            range: lsp_types::Range {
                                                start: lsp_types::Position {
                                                    line: 0,
                                                    character: 0,
                                                },
                                                end: lsp_types::Position {
                                                    line: contents.lines().count() as u32,
                                                    character: contents
                                                        .lines()
                                                        .last()
                                                        .map_or(0, |line| line.len() as u32),
                                                },
                                            },
                                            new_text: formatted,
                                        };
                                        respond::<Formatting>(
                                            Ok(Some(vec![edit])),
                                            request_id,
                                            &connection,
                                        )?;
                                    }
                                    Err(_parse_error) => {
                                        respond::<SemanticTokensFullRequest>(
                                            // NOTE: responding with the error like this is
                                            // actually just annoying...(at least in vscode)
                                            //
                                            //Err(lsp_server::ResponseError {
                                            //    code: lsp_server::ErrorCode::ParseError as i32,
                                            //    message: format!("{:?}", parse_error),
                                            //    data: None,
                                            //}),
                                            Ok(None),
                                            request_id,
                                            &connection,
                                        )?;
                                    }
                                }
                            } else {
                                respond::<SemanticTokensFullRequest>(
                                    Err(lsp_server::ResponseError {
                                        code: lsp_server::ErrorCode::InternalError as i32,
                                        message: format!(
                                            "no tree for {}",
                                            params.text_document.uri
                                        ),
                                        data: None,
                                    }),
                                    request_id,
                                    &connection,
                                )?;
                            }
                            continue 'msg_loop;
                        }
                        Err(_req) => (),
                    },
                };
            }
            lsp_server::Message::Response(_response) => {
                //debug!("response: {:?}", resp);
            }
            lsp_server::Message::Notification(not) => {
                match cast_notification::<lsp_types::notification::DidOpenTextDocument>(not) {
                    Ok(params) => {
                        trees.insert(params.text_document.uri, params.text_document.text);
                    }
                    Err(not) => match cast_notification::<
                        lsp_types::notification::DidChangeTextDocument,
                    >(not)
                    {
                        Ok(params) => {
                            for change in params.content_changes {
                                let source = change.text; // because TextDocumentSyncKind::FULL
                                trees.update(&params.text_document.uri, source);
                            }
                        }
                        Err(_not) => (),
                    },
                }
            }
        }
    }
    Ok(())
}

/// Parsed trees, updated on text document change notifications.
struct Trees(HashMap<Url, (tree_sitter::Tree, String)>);

impl Trees {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn insert(&mut self, url: Url, source: String) {
        let mut parser = init_parser();
        if let Some(tree) = parser.parse(&source, None) {
            log::debug!("tree inserted for {}", url);
            self.0.insert(url, (tree, source));
        } else {
            log::error!("parse result was None for {}", url)
        }
    }

    // TODO: make this INCREMENTAL
    fn update(&mut self, url: &Url, source: String) {
        let mut parser = init_parser();
        if let Some(tree) = parser.parse(&source, None) {
            log::debug!("tree updated for {}", url);
            self.0.insert(url.clone(), (tree, source));
        } else {
            log::warn!("parse result was None for {}", url)
        }
    }

    fn get(&self, url: &Url) -> Option<&(tree_sitter::Tree, String)> {
        self.0.get(url)
    }
}

// Panic if the parser fails to initialise, as this really shouldn't happen.
fn init_parser() -> tree_sitter::Parser {
    try_init_parser().unwrap_or_else(|lang_err| {
        panic!(
            "Error initialising tree-sitter parser with ditto language: {}",
            lang_err
        )
    })
}

fn try_init_parser() -> Result<tree_sitter::Parser, tree_sitter::LanguageError> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(tree_sitter_ditto::language())?;
    Ok(parser)
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
