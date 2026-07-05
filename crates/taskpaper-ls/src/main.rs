//! taskpaper-ls: a lightweight language server for TaskPaper todo lists.
//!
//! Speaks LSP over stdio, full-document sync (TaskPaper files are small).
//! Features: due-date diagnostics, project-count and due-countdown inlay
//! hints, hover, tag completion, code actions (toggle @done/@cancelled,
//! archive finished items, sort by due date, task/note conversion), tag
//! rename, workspace project symbols, and formatting.

// `WorkspaceEdit.changes` is defined by lsp-types as HashMap keyed by `Uri`,
// which clippy flags for interior mutability; the keys are never mutated.
#![allow(clippy::mutable_key_type)]

mod actions;
mod analysis;
mod dates;
mod model;
mod util;
mod workspace;

use std::collections::HashMap;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _, PublishDiagnostics,
};
use lsp_types::request::{
    CodeActionRequest, Completion, Formatting, HoverRequest, InlayHintRequest,
    PrepareRenameRequest, Rename, Request as _, WorkspaceSymbolRequest,
};
use lsp_types::{
    CodeActionOrCommand, CodeActionParams, CodeActionProviderCapability, CompletionOptions,
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, DocumentFormattingParams, HoverParams,
    InitializeParams, InlayHintParams, OneOf, PrepareRenameResponse, PublishDiagnosticsParams,
    RenameOptions, RenameParams, ServerCapabilities, TextDocumentPositionParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri, WorkspaceEdit, WorkspaceSymbolParams,
    WorkspaceSymbolResponse,
};

type Error = Box<dyn std::error::Error + Sync + Send>;

struct Server {
    docs: HashMap<String, model::Doc>,
    index: workspace::Index,
}

fn main() -> Result<(), Error> {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["@".into(), "(".into()]),
            ..CompletionOptions::default()
        }),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        inlay_hint_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: Default::default(),
        })),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        ..ServerCapabilities::default()
    })?;
    let init_value = connection.initialize(capabilities)?;
    let init: InitializeParams = serde_json::from_value(init_value)?;

    #[allow(deprecated)] // root_uri: still what many clients send
    let root = init
        .workspace_folders
        .as_ref()
        .and_then(|folders| folders.first())
        .map(|folder| &folder.uri)
        .or(init.root_uri.as_ref())
        .and_then(workspace::path_of_uri);

    let mut server = Server {
        docs: HashMap::new(),
        index: workspace::Index::build(root),
    };

    // Surface diagnostics for every TaskPaper file in the workspace up
    // front, not just the ones that get opened. (Connection::initialize has
    // already consumed the `initialized` notification at this point.)
    publish_workspace_diagnostics(&connection, &server)?;

    main_loop(connection, &mut server)?;
    io_threads.join()?;
    Ok(())
}

fn main_loop(connection: Connection, server: &mut Server) -> Result<(), Error> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                handle_request(&connection, server, req)?;
            }
            Message::Notification(not) => handle_notification(&connection, server, not)?,
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn doc_and_pos<'a>(
    server: &'a Server,
    pos: &TextDocumentPositionParams,
) -> Option<(&'a model::Doc, usize, u32)> {
    let doc = server.docs.get(pos.text_document.uri.as_str())?;
    Some((doc, pos.position.line as usize, pos.position.character))
}

fn handle_request(connection: &Connection, server: &mut Server, req: Request) -> Result<(), Error> {
    let response = match req.method.as_str() {
        HoverRequest::METHOD => {
            let (id, params) = req.extract::<HoverParams>(HoverRequest::METHOD)?;
            let hover = doc_and_pos(server, &params.text_document_position_params)
                .and_then(|(doc, row, col)| analysis::hover(doc, row, col, dates::today()));
            Response::new_ok(id, hover)
        }
        Completion::METHOD => {
            let (id, params) = req.extract::<CompletionParams>(Completion::METHOD)?;
            let items = doc_and_pos(server, &params.text_document_position)
                .map(|(doc, row, col)| workspace::completions(doc, &server.index, row, col))
                .unwrap_or_default();
            Response::new_ok(id, CompletionResponse::Array(items))
        }
        CodeActionRequest::METHOD => {
            let (id, params) = req.extract::<CodeActionParams>(CodeActionRequest::METHOD)?;
            let uri = params.text_document.uri;
            let actions: Vec<CodeActionOrCommand> = server
                .docs
                .get(uri.as_str())
                .map(|doc| {
                    actions::code_actions(
                        doc,
                        &uri,
                        params.range.start.line as usize,
                        dates::today(),
                    )
                    .into_iter()
                    .map(CodeActionOrCommand::CodeAction)
                    .collect()
                })
                .unwrap_or_default();
            Response::new_ok(id, actions)
        }
        InlayHintRequest::METHOD => {
            let (id, params) = req.extract::<InlayHintParams>(InlayHintRequest::METHOD)?;
            let hints = server
                .docs
                .get(params.text_document.uri.as_str())
                .map(|doc| {
                    analysis::inlay_hints(
                        doc,
                        params.range.start.line as usize,
                        params.range.end.line as usize,
                        dates::today(),
                    )
                });
            Response::new_ok(id, hints)
        }
        Formatting::METHOD => {
            let (id, params) = req.extract::<DocumentFormattingParams>(Formatting::METHOD)?;
            let edits = server
                .docs
                .get(params.text_document.uri.as_str())
                .map(actions::format);
            Response::new_ok(id, edits)
        }
        PrepareRenameRequest::METHOD => {
            let (id, params) =
                req.extract::<TextDocumentPositionParams>(PrepareRenameRequest::METHOD)?;
            let range = doc_and_pos(server, &params)
                .and_then(|(doc, row, col)| workspace::prepare_rename(doc, row, col))
                .map(PrepareRenameResponse::Range);
            Response::new_ok(id, range)
        }
        Rename::METHOD => {
            let (id, params) = req.extract::<RenameParams>(Rename::METHOD)?;
            let edit =
                doc_and_pos(server, &params.text_document_position).and_then(|(doc, row, col)| {
                    let line = doc.lines.get(row)?;
                    let byte = util::byte_from_utf16(line, col);
                    let (i, t) = doc.tag_at(row, byte)?;
                    let old = doc.items[i].tags[t].name.clone();
                    let changes =
                        workspace::rename(&server.index, &server.docs, &old, &params.new_name);
                    Some(WorkspaceEdit {
                        changes: Some(changes),
                        ..WorkspaceEdit::default()
                    })
                });
            Response::new_ok(id, edit)
        }
        WorkspaceSymbolRequest::METHOD => {
            let (id, params) =
                req.extract::<WorkspaceSymbolParams>(WorkspaceSymbolRequest::METHOD)?;
            let symbols = server.index.symbols(&params.query);
            Response::new_ok(id, WorkspaceSymbolResponse::Flat(symbols))
        }
        _ => Response::new_err(
            req.id,
            lsp_server::ErrorCode::MethodNotFound as i32,
            format!("unhandled method: {}", req.method),
        ),
    };
    connection.sender.send(Message::Response(response))?;
    Ok(())
}

fn handle_notification(
    connection: &Connection,
    server: &mut Server,
    not: Notification,
) -> Result<(), Error> {
    match not.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = not.extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)?;
            let d = params.text_document;
            let doc = model::parse(&d.text);
            publish_diagnostics(connection, &d.uri, &doc, Some(d.version))?;
            index_doc(server, &d.uri, &d.text);
            server.docs.insert(d.uri.as_str().to_owned(), doc);
        }
        DidChangeTextDocument::METHOD => {
            let params =
                not.extract::<DidChangeTextDocumentParams>(DidChangeTextDocument::METHOD)?;
            // Full sync: the last change carries the whole document.
            let Some(change) = params.content_changes.into_iter().next_back() else {
                return Ok(());
            };
            let uri = params.text_document.uri;
            let doc = model::parse(&change.text);
            publish_diagnostics(connection, &uri, &doc, Some(params.text_document.version))?;
            index_doc(server, &uri, &change.text);
            server.docs.insert(uri.as_str().to_owned(), doc);
        }
        DidSaveTextDocument::METHOD => {
            let params = not.extract::<DidSaveTextDocumentParams>(DidSaveTextDocument::METHOD)?;
            if let Some(path) = workspace::path_of_uri(&params.text_document.uri) {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    server.index.update(&path, &text);
                }
            }
        }
        DidCloseTextDocument::METHOD => {
            let params = not.extract::<DidCloseTextDocumentParams>(DidCloseTextDocument::METHOD)?;
            server.docs.remove(params.text_document.uri.as_str());
            // Keep the closed file's diagnostics alive from its on-disk
            // state (so the project diagnostics panel stays a cross-file
            // view); clear them only if the file is gone.
            let uri = params.text_document.uri;
            let doc = workspace::path_of_uri(&uri)
                .and_then(|path| std::fs::read_to_string(path).ok())
                .map(|text| model::parse(&text))
                .unwrap_or_default();
            publish_diagnostics(connection, &uri, &doc, None)?;
        }
        _ => {}
    }
    Ok(())
}

/// Publish diagnostics for every indexed workspace file that isn't open
/// (open documents are covered by didOpen/didChange with buffer content).
fn publish_workspace_diagnostics(connection: &Connection, server: &Server) -> Result<(), Error> {
    for path in server.index.files() {
        let Some(uri) = workspace::uri_of_path(path) else {
            continue;
        };
        if server.docs.contains_key(uri.as_str()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        publish_diagnostics(connection, &uri, &model::parse(&text), None)?;
    }
    Ok(())
}

fn index_doc(server: &mut Server, uri: &Uri, text: &str) {
    if let Some(path) = workspace::path_of_uri(uri) {
        server.index.update(&path, text);
    }
}

fn publish_diagnostics(
    connection: &Connection,
    uri: &Uri,
    doc: &model::Doc,
    version: Option<i32>,
) -> Result<(), Error> {
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics: analysis::diagnostics(doc, dates::today()),
        version,
    };
    connection
        .sender
        .send(Message::Notification(Notification::new(
            PublishDiagnostics::METHOD.into(),
            params,
        )))?;
    Ok(())
}
