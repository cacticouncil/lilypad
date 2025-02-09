use egui::Color32;

pub mod blocks_theme;

pub mod syntax {
    use egui::Color32;

    use super::one_dark::*;

    pub const FUNCTION: Color32 = BLUE;
    pub const FUNCTION_BUILT_IN: Color32 = CYAN;
    pub const KEYWORD: Color32 = MAGENTA;
    pub const OPERATOR: Color32 = WHITE;
    pub const PROPERTY: Color32 = LIGHT_RED;
    pub const INTERPOLATION_SURROUNDING: Color32 = LIGHT_YELLOW;
    pub const STRING: Color32 = GREEN;
    pub const TYPE: Color32 = LIGHT_YELLOW;
    pub const VARIABLE: Color32 = LIGHT_RED;
    pub const CONSTRUCTOR: Color32 = BLUE;
    pub const CONSTANT: Color32 = DARK_YELLOW;
    pub const LITERAL: Color32 = DARK_YELLOW;
    pub const ESCAPE_SEQUENCE: Color32 = CYAN;
    pub const COMMENT: Color32 = COMMENT_GREY;

    pub const DEFAULT: Color32 = WHITE;
}

pub mod diagnostic {
    use egui::Color32;

    pub const ERROR: Color32 = Color32::from_rgb(0xC2, 0x40, 0x38);
    pub const WARNING: Color32 = Color32::from_rgb(0xD1, 0x9A, 0x66);
    pub const INFO: Color32 = Color32::from_rgb(0x37, 0x94, 0xFF);
    pub const HINT: Color32 = Color32::from_rgb(0xA0, 0xA0, 0xA0);
}

pub const INTERFACE_TEXT: Color32 = one_dark::WHITE;
pub const BACKGROUND: Color32 = one_dark::BLACK;
pub const CURSOR: Color32 = Color32::from_rgb(0x52, 0x8B, 0xFF);
pub const SELECTION: Color32 = Color32::from_rgb(0x3D, 0x45, 0x55);
pub const PSEUDO_SELECTION: Color32 = Color32::from_rgba_premultiplied(0x32, 0x09, 0x03, 0x50);
pub const POPUP_BACKGROUND: Color32 = Color32::from_rgb(0x1E, 0x22, 0x27);
pub const LINE_NUMBERS: Color32 = one_dark::GUTTER_GREY;

pub const BREAKPOINT: Color32 = Color32::from_rgb(0xFF, 0x45, 0x45);
pub const PREVIEW_BREAKPOINT: Color32 = Color32::from_rgb(0x71, 0x1F, 0x1C);
pub const STACK_FRAME_SELECTED: Color32 = Color32::from_rgb(0x00, 0xFF, 0x00);
pub const STACK_FRAME_DEEPEST: Color32 = Color32::from_rgb(0xFF, 0xFF, 0x00);

#[allow(dead_code)]
mod one_dark {
    use egui::Color32;

    pub const BLACK: Color32 = Color32::from_rgb(40, 44, 52);
    pub const WHITE: Color32 = Color32::from_rgb(171, 178, 191);
    pub const LIGHT_RED: Color32 = Color32::from_rgb(224, 108, 117);
    pub const DARK_RED: Color32 = Color32::from_rgb(190, 80, 70);
    pub const GREEN: Color32 = Color32::from_rgb(152, 195, 121);
    pub const LIGHT_YELLOW: Color32 = Color32::from_rgb(229, 192, 123);
    pub const DARK_YELLOW: Color32 = Color32::from_rgb(209, 154, 102);
    pub const BLUE: Color32 = Color32::from_rgb(97, 175, 239);
    pub const MAGENTA: Color32 = Color32::from_rgb(198, 120, 221);
    pub const CYAN: Color32 = Color32::from_rgb(86, 182, 194);
    pub const GUTTER_GREY: Color32 = Color32::from_rgb(76, 82, 99);
    pub const COMMENT_GREY: Color32 = Color32::from_rgb(92, 99, 112);
}
