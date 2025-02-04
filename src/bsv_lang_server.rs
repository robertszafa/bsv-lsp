use tower_lsp::lsp_types::*;
// To run the bsc compiler.
use std::process::Command;

/// Given a bluespec text decoumnt 'fname', compile it with 'bsc -sim fname',
/// and return the diagnostics from bsc.
pub fn collect_diagnostics(file_url: &Url) -> Option<Vec<Diagnostic>> {
    let fname = file_url.path();
    let bsc_out = try_bsc_compile(fname)?;
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
            diagnostics.push(create_diagnostics(
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
            curr_diag_line_nr = get_line_nr(line).unwrap_or(0);
            curr_diag_col_nr = get_column_nr(line).unwrap_or(0);
        } else {
            // Body of diagnostic message.
            curr_diag_msg.push_str(line.trim_start());
            curr_diag_msg.push(' ');
        }
    }

    // Final diagnostic.
    if !curr_diag_msg.is_empty() {
        diagnostics.push(create_diagnostics(
            curr_diag_line_nr,
            curr_diag_col_nr,
            curr_diag_severity,
            curr_diag_msg,
        ));
    }

    Some(diagnostics)
}

/// Construct Diagnostic object.
fn create_diagnostics(
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
fn get_line_nr(diagnostic_msg: &str) -> Option<u32> {
    let (_, line_substr) = diagnostic_msg.split_once(", line ")?;
    let (line_str, _) = line_substr.split_once(",")?;
    let line = line_str.parse::<u32>().ok()?;
    Some(line - 1)
}

/// Given a bsc error/warning string, return the column/character number.
///     Example line error: Error: "Top.bsv", line 98, column 3: (P0005)
fn get_column_nr(diagnostic_msg: &str) -> Option<u32> {
    let (_, col_substr) = diagnostic_msg.split_once(", column ")?;
    let (col_str, _) = col_substr.split_once(":")?;
    let col = col_str.parse::<u32>().ok()?;
    Some(col - 1)
}

fn try_bsc_compile(fname: &str) -> Option<String> {
    let bsc_out = Command::new("bsc")
        .arg("-sim") // To get more diagnostics, e.g., rule conflicts
        .arg(fname)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&bsc_out.stderr).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_diagnostics() {
        let project_root = std::env::var("CARGO_MANIFEST_DIR").expect("to get project root dir.");
        let bsv_test_file_path = project_root + "/bsv_test_files/Top.bsv";
        let bsv_test_url = Url::from_file_path(bsv_test_file_path).expect("to get url");
        let diagnostics = collect_diagnostics(&bsv_test_url).expect("some diagnostics");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].range.start.line, 111);
        assert_eq!(diagnostics[0].range.start.character, 2);
        assert_eq!(diagnostics[0].range.end.line, 111);
        assert_eq!(diagnostics[0].range.end.character, 2);
        assert!(diagnostics[0].message.contains("Unexpected identifier "));
    }
}


