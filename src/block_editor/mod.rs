use egui::Vec2;
use egui::{
    text::Fonts, CentralPanel, FontFamily, FontId, Frame, Pos2, Rect, Sense, SidePanel, Widget,
};
use ropey::Rope;
use source::Source;

use crate::lang::Language;
use crate::lsp::completion::VSCodeCompletionItem;
use crate::theme::blocks_theme::BlocksTheme;
use crate::{theme, vscode};

mod block_drawer;
mod dragging;
mod rope_ext;
pub mod source;
mod text_drawer;
pub mod text_editor;
pub mod text_range;

pub use block_drawer::BlockType;

use self::dragging::block_palette::BlockPalette;
use self::dragging::loose_block::LooseBlock;
use self::source::TextEdit;
use self::text_editor::StackFrameLines;
use self::text_editor::TextEditor;
use self::text_range::TextRange;
use crate::lsp::diagnostics::{Diagnostic, VSCodeCodeAction};

/// padding around edges of entire editor
const OUTER_PAD: f32 = 16.0;

/// left padding on text (to position it nicer within the blocks)
const TEXT_L_PAD: f32 = 2.0;

/// width for the line number gutter
const GUTTER_WIDTH: f32 = 30.0;

/// convenience constant for all the padding that impacts text layout
const TOTAL_TEXT_X_OFFSET: f32 = OUTER_PAD + GUTTER_WIDTH + TEXT_L_PAD;

const SHOW_ERROR_BLOCK_OUTLINES: bool = false;

pub struct BlockEditor {
    /// the source code being edited and the associated language
    source: Source,

    /// the color theme for the blocks
    blocks_theme: BlocksTheme,

    /// the font used for code
    font: MonospaceFont,

    /// the editor widget
    text_editor: TextEditor,

    /// the palette of blocks that can be dragged into the editor
    block_palette: BlockPalette,

    /// text that is currently getting dragged
    drag_block: Option<DragSession>,

    /// the popup for the currently dragged block
    dragging_popup: Option<LooseBlock>,
}

#[derive(Debug)]
pub struct DragSession {
    text: String,

    /// point within the block that it is dragged by
    offset: Pos2,
}

#[allow(dead_code)]
pub enum ExternalCommand {
    // setup
    SetText(String),
    SetFile { name: String, contents: String },
    SetBlocksTheme(BlocksTheme),
    SetFont(String, f32),

    // external edits
    ApplyEdit(TextEdit<'static>),

    // lsp connection
    SetDiagnostics(Vec<Diagnostic>),
    SetQuickFix(usize, Vec<VSCodeCodeAction>),
    SetCompletions(Vec<VSCodeCompletionItem>),

    // debugging
    SetBreakpoints(Vec<usize>),
    SetStackFrame(StackFrameLines),

    // undo/redo
    Undo,
    Redo,
}

pub struct MonospaceFont {
    /// The font size and family
    id: FontId,

    /// The size in pixels of a single character
    size: Vec2,
}

impl MonospaceFont {
    /// Create a new monospace font.
    /// Note: does not calculate the size yet. Must also call `calculate_size` before using..
    pub fn new(_family: &str, size: f32) -> Self {
        // TODO: support custom font families
        let id = FontId::new(size, FontFamily::Monospace);
        Self {
            id,
            size: Vec2::ZERO,
        }
    }

    pub fn calculate_size(&mut self, fonts: &Fonts) {
        self.size = Vec2::new(fonts.glyph_width(&self.id, 'A'), fonts.row_height(&self.id));
    }
}

impl BlockEditor {
    pub fn new(file_name: &str, blocks_theme: &str, font: MonospaceFont) -> Self {
        let lang = Language::for_file(file_name);
        vscode::log_event(
            "opened-file",
            std::collections::HashMap::from([("lang", lang.config.name)]),
        );
        BlockEditor {
            source: Source::new(Rope::new(), lang),
            blocks_theme: BlocksTheme::for_str(blocks_theme),
            font,
            text_editor: TextEditor::new(),
            block_palette: BlockPalette::new(),
            drag_block: None,
            dragging_popup: None,
        }
    }

    pub fn widget<'a>(&'a mut self, external_commands: &'a [ExternalCommand]) -> impl Widget + 'a {
        move |ui: &mut egui::Ui| -> egui::Response {
            // trigger started on the first frame
            if ui.ctx().frame_nr() == 0 {
                vscode::started();
            }

            // calculate the size of a character at the start of the frame
            // since it can change with pixels_per_point
            ui.fonts(|f| self.font.calculate_size(f));

            // handle external commands
            for command in external_commands.iter() {
                match command {
                    ExternalCommand::SetText(text) => {
                        self.source.set_text(Rope::from_str(text));
                    }
                    ExternalCommand::SetFile { name, contents } => {
                        let language = Language::for_file(name);
                        self.source = Source::new(Rope::from_str(contents), language);
                        self.block_palette
                            .populate(&mut self.source.lang, &self.font)
                    }
                    ExternalCommand::SetBlocksTheme(theme) => {
                        self.blocks_theme = *theme;
                    }
                    ExternalCommand::SetFont(font_name, font_size) => {
                        self.font = MonospaceFont::new(font_name, *font_size);
                        ui.fonts(|f| self.font.calculate_size(f));
                    }
                    _ => {}
                }
            }

            if !self.block_palette.is_populated() {
                self.block_palette
                    .populate(&mut self.source.lang, &self.font);
            }

            let response =
                ui.allocate_response(ui.available_size(), Sense::focusable_noninteractive());

            self.editor_contents(ui, external_commands);

            self.draw_dragged_block(ui);

            response
        }
    }

    fn editor_contents(&mut self, ui: &mut egui::Ui, external_commands: &[ExternalCommand]) {
        let palette_size = self.block_palette.find_size();
        SidePanel::right("palette_panel")
            .exact_width(palette_size.x)
            .show_separator_line(false)
            .resizable(false)
            .frame(Frame::none())
            .show(ui.ctx(), |ui| {
                ui.add(self.block_palette.widget(
                    &mut self.drag_block,
                    self.blocks_theme,
                    &self.font,
                ));
            });

        CentralPanel::default()
            .frame(Frame::none())
            .show(ui.ctx(), |ui| {
                ui.add(self.text_editor.widget(
                    &mut self.source,
                    &mut self.drag_block,
                    external_commands,
                    self.blocks_theme,
                    &self.font,
                ));
            });
    }

    fn draw_dragged_block(&mut self, ui: &mut egui::Ui) {
        if let Some(drag_block) = &mut self.drag_block {
            // show dragging cursor
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

            // create the dragging popup if this is the first frame of the drag
            if self.dragging_popup.is_none() {
                self.dragging_popup = Some(LooseBlock::new(
                    &drag_block.text,
                    40.0,
                    &mut self.source.lang,
                    &self.font,
                ));
            }

            if let Some(dragging_popup) = &mut self.dragging_popup {
                // add the dragging pop up where the mouse is
                if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let painter = ui.painter();
                    let origin = mouse_pos - drag_block.offset;
                    painter.rect_filled(
                        Rect::from_min_size(origin.to_pos2(), dragging_popup.min_size()),
                        0.0,
                        theme::BACKGROUND.gamma_multiply(0.75),
                    );
                    dragging_popup.draw(
                        origin,
                        dragging_popup.min_size().x,
                        self.blocks_theme,
                        &self.font,
                        painter,
                    )
                }
            }
        } else {
            // clear the dragging popup if the drag is over
            self.dragging_popup = None;
        }
    }
}
