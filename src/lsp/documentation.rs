use serde::{Deserialize, Serialize};

use crate::block_editor::text_range::{TextPoint, TextRange};

use crate::vscode;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Documentation {
    pub message: String,
    pub range: TextRange,
}

impl Documentation {
    pub fn request_hover(&self) {
        crate::vscode::request_hover(self.range.start.line, self.range.start.col);
    }

    #[allow(dead_code)]
    pub fn example() -> Documentation {
        Documentation {
            message: "example Documentation".to_string(),
            range: TextRange::new(TextPoint::new(2, 18), TextPoint::new(2, 25)),
        }
    }
}
#[derive(Deserialize, Debug, Clone)]
pub struct VSCodeHoverItem {
    content: String,
    #[serde(rename = "edit")]
    workspace_edit: Option<serde_json::Value>,
    command: Option<VSCodeCommand>,
}

impl VSCodeHoverItem {
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
