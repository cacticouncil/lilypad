use druid::Color;

pub mod blocks_theme;

pub mod syntax {
    use druid::Color;

    use super::one_dark::*;

    pub const FUNCTION: Color = BLUE;
    pub const FUNCTION_BUILT_IN: Color = CYAN;
    pub const KEYWORD: Color = MAGENTA;
    pub const OPERATOR: Color = WHITE;
    pub const PROPERTY: Color = LIGHT_RED;
    pub const INTERPOLATION_SURROUNDING: Color = LIGHT_YELLOW;
    pub const STRING: Color = GREEN;
    pub const TYPE: Color = LIGHT_YELLOW;
    pub const VARIABLE: Color = LIGHT_RED;
    pub const CONSTRUCTOR: Color = BLUE;
    pub const CONSTANT: Color = DARK_YELLOW;
    pub const LITERAL: Color = DARK_YELLOW;
    pub const ESCAPE_SEQUENCE: Color = CYAN;
    pub const COMMENT: Color = COMMENT_GREY;

    pub const DEFAULT: Color = WHITE;
}

pub mod diagnostic {
    use druid::Color;

    pub const ERROR: Color = Color::rgb8(0xC2, 0x40, 0x38);
    pub const WARNING: Color = Color::rgb8(0xD1, 0x9A, 0x66);
    pub const INFO: Color = Color::rgb8(0x37, 0x94, 0xFF);
    pub const HINT: Color = Color::rgb8(0xA0, 0xA0, 0xA0);
}

pub const INTERFACE_TEXT: Color = one_dark::WHITE;
pub const BACKGROUND: Color = one_dark::BLACK;
pub const CURSOR: Color = Color::rgb8(0x52, 0x8B, 0xFF);
pub const SELECTION: Color = Color::rgb8(0x3D, 0x45, 0x55);
pub const PSEUDO_SELECTION: Color = Color::rgba8(0xA0, 0x1C, 0x08, 0x40);
pub const POPUP_BACKGROUND: Color = Color::rgb8(0x1E, 0x22, 0x27);
pub const LINE_NUMBERS: Color = one_dark::GUTTER_GREY;

pub const BREAKPOINT: Color = Color::rgb8(0xFF, 0x45, 0x45);
pub const STACK_FRAME_SELECTED: Color = Color::rgb8(0x00, 0xFF, 0x00);
pub const STACK_FRAME_DEEPEST: Color = Color::rgb8(0xFF, 0xFF, 0x00);

#[allow(dead_code)]
mod one_dark {
    use druid::Color;

    pub const BLACK: Color = Color::rgb8(40, 44, 52);
    pub const WHITE: Color = Color::rgb8(171, 178, 191);
    pub const LIGHT_RED: Color = Color::rgb8(224, 108, 117);
    pub const DARK_RED: Color = Color::rgb8(190, 80, 70);
    pub const GREEN: Color = Color::rgb8(152, 195, 121);
    pub const LIGHT_YELLOW: Color = Color::rgb8(229, 192, 123);
    pub const DARK_YELLOW: Color = Color::rgb8(209, 154, 102);
    pub const BLUE: Color = Color::rgb8(97, 175, 239);
    pub const MAGENTA: Color = Color::rgb8(198, 120, 221);
    pub const CYAN: Color = Color::rgb8(86, 182, 194);
    pub const GUTTER_GREY: Color = Color::rgb8(76, 82, 99);
    pub const COMMENT_GREY: Color = Color::rgb8(92, 99, 112);
}
