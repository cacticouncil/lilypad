use egui::Color32;
use serde::{Deserialize, Serialize};

use crate::block_editor::text_range::{TextPoint, TextRange};
use crate::vscode;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub range: TextRange,
    pub severity: DiagnosticSeverity,
    pub source: Option<String>,
    #[serde(skip, default)]
    pub id: usize,
}

impl Diagnostic {
    pub fn request_fixes(&self) {
        crate::vscode::request_quick_fixes(self.id, self.range.start.line, self.range.start.col);
    }

    #[allow(dead_code)]
    pub fn example() -> Diagnostic {
        Diagnostic {
            message: "example diagnostic".to_string(),
            range: TextRange::new(TextPoint::new(2, 18), TextPoint::new(2, 25)),
            severity: DiagnosticSeverity::Error,
            source: Some("example".to_string()),
            id: 0,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, PartialOrd)]
pub enum DiagnosticSeverity {
    Error = 3,
    Warning = 2,
    Information = 1,
    Hint = 0,
}

impl DiagnosticSeverity {
    pub fn color(&self) -> Color32 {
        use crate::theme::diagnostic::*;
        use DiagnosticSeverity::*;

        match self {
            Error => ERROR,
            Warning => WARNING,
            Information => INFO,
            Hint => HINT,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct VSCodeCodeAction {
    pub title: String,
    #[serde(rename = "edit")]
    workspace_edit: Option<serde_json::Value>,
    command: Option<VSCodeCommand>,
}

impl VSCodeCodeAction {
    pub fn run(&self) {
        // run workspace edit then command
        if let Some(workspace_edit) = &self.workspace_edit {
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            vscode::execute_workspace_edit(
                workspace_edit.serialize(&serializer).unwrap_or_default(),
            );
        }
        if let Some(command) = &self.command {
            command.run();
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct VSCodeCommand {
    command: String,
    arguments: serde_json::Value,
}

impl VSCodeCommand {
    pub fn run(&self) {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        vscode::execute_command(
            self.command.clone(),
            self.arguments.serialize(&serializer).unwrap_or_default(),
        );
    }
}
