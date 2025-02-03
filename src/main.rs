// use std::fs;
// use std::io::Write;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

// To run the bsc compiler.
use std::process::Command;

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                // completion_provider: Some(CompletionOptions {
                //     resolve_provider: Some(false),
                //     trigger_characters: Some(vec![".".to_string()]),
                //     work_done_progress_options: Default::default(),
                //     all_commit_characters: None,
                //     ..Default::default()
                // }),
                // execute_command_provider: Some(ExecuteCommandOptions {
                //     commands: vec!["dummy.do_something".to_string()],
                //     work_done_progress_options: Default::default(),
                // }),
                // workspace: Some(WorkspaceServerCapabilities {
                //     workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                //         supported: Some(true),
                //         change_notifications: Some(OneOf::Left(true)),
                //     }),
                //     file_operations: None,
                // }),
                ..ServerCapabilities::default()
            }
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "BSV LSP has been initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed!")
            .await;
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client
            .log_message(MessageType::INFO, "configuration changed!")
            .await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client
            .log_message(MessageType::INFO, "watched files have changed!")
            .await;
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(MessageType::INFO, "command executed!")
            .await;

        match self.client.apply_edit(WorkspaceEdit::default()).await {
            Ok(res) if res.applied => self.client.log_message(MessageType::INFO, "applied").await,
            Ok(_) => self.client.log_message(MessageType::INFO, "rejected").await,
            Err(err) => self.client.log_message(MessageType::ERROR, err).await,
        }

        Ok(None)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        if let Some(diagnostics) = self.collect_diagnostics(&params.text_document) {
            self.client
                .publish_diagnostics(params.text_document.uri, diagnostics, Option::None)
                .await;
        } else {
            self.client
                .log_message(MessageType::ERROR, "Failed to compile and get bsc diagnostics.")
                .await;
        }
    }

    async fn did_change(&self, _: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file has changed!")
            .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let doc_item = TextDocumentItem {
            uri: params.text_document.uri,
            text: String::from(""),
            version: 1,
            language_id: String::from("bsv"),
        };
        if let Some(diagnostics) = self.collect_diagnostics(&doc_item) {
            self.client
                .publish_diagnostics(doc_item.uri, diagnostics, Option::None)
                .await;
        } else {
            self.client
                .log_message(MessageType::ERROR, "Failed to compile and get bsc diagnostics.")
                .await;
        }
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file closed!")
            .await;
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
    }
}

impl Backend {
    /// Construct Diagnostic object.
    fn create_diagnostics(
        &self,
        line: u32,
        col: u32,
        severity: DiagnosticSeverity,
        msg: String,
    ) -> Diagnostic {
        let diag_range = Range::new(
            Position {
                line,
                character: col,
            },
            Position {
                line,
                character: col,
            },
        );
        Diagnostic {
            range: diag_range,
            severity: Some(severity),
            message: msg,
            ..Diagnostic::default()
        }
    }

    /// Given a bsc error/warning string, return the line number.
    ///     Example line error: Error: "Top.bsv", line 98, column 3: (P0005)
    fn get_line_nr(&self, diagnostic_msg: &str) -> Option<u32> {
        let (_, line_substr) = diagnostic_msg.split_once(", line ")?;
        let (line_str, _) = line_substr.split_once(",")?;
        let line = line_str.parse::<u32>().ok()?;
        Some(line - 1)
    }

    /// Given a bsc error/warning string, return the column/character number.
    ///     Example line error: Error: "Top.bsv", line 98, column 3: (P0005)
    fn get_column_nr(&self, diagnostic_msg: &str) -> Option<u32> {
        let (_, col_substr) = diagnostic_msg.split_once(", column ")?;
        let (col_str, _) = col_substr.split_once(":")?;
        let col = col_str.parse::<u32>().ok()?;
        Some(col - 1)
    }

    fn try_bsc_compile(&self, fname: &str) -> Option<String> {
        let bsc_out = Command::new("bsc")
            .arg("-sim") // To get more diagnostics, e.g., rule conflicts
            .arg(fname)
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&bsc_out.stderr).to_string())
    }

    /// Given a bluespec text decoumnt 'fname', compile it with 'bsc -sim fname',
    /// and return the diagnostics from bsc.
    fn collect_diagnostics(&self, params: &TextDocumentItem) -> Option<Vec<Diagnostic>> {
        let fname = params.uri.path();
        let bsc_out = self.try_bsc_compile(fname)?;
        // let fcontents = fs::read_to_string(fname).expect("to read {fname}.");

        // Go through all lines outputted by bsc and collect all diagnostics.
        let mut diagnostics: Vec<Diagnostic> = Vec::new();
        let mut curr_diag_msg = String::new();
        let mut curr_diag_line_nr = 0;
        let mut curr_diag_col_nr = 0;
        let mut curr_diag_severity = DiagnosticSeverity::ERROR;
        for line in bsc_out.lines() {
            // If we start a new diagnostic, then push any prev diagnostic to our vector.
            let is_start_of_new_diag = line.contains("Error: ") || line.contains("Warning: ");
            if is_start_of_new_diag && !curr_diag_msg.is_empty() {
                diagnostics.push(self.create_diagnostics(
                    curr_diag_line_nr,
                    curr_diag_col_nr,
                    curr_diag_severity,
                    curr_diag_msg.clone(),
                ));
            }

            // Update info about next diag, if this is its start. Else collect msg body of existing.
            if is_start_of_new_diag {
                curr_diag_severity = if line.contains("Error: ") {
                    DiagnosticSeverity::ERROR
                } else {
                    DiagnosticSeverity::WARNING
                };
                curr_diag_msg.clear();
                curr_diag_line_nr = self.get_line_nr(line).unwrap_or(0);
                curr_diag_col_nr = self.get_column_nr(line).unwrap_or(0);
            } else {
                // Body of diagnostic message.
                curr_diag_msg.push_str(line.trim_start());
                curr_diag_msg.push(' ');
            }
        }

        // Final diagnostic.
        if !curr_diag_msg.is_empty() {
            diagnostics.push(self.create_diagnostics(
                curr_diag_line_nr,
                curr_diag_col_nr,
                curr_diag_severity,
                curr_diag_msg,
            ));
        }

        Some(diagnostics)
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
