pub mod syntax {
    use druid::Color;

    use super::one_dark;

    pub const FUNCTION: Color = one_dark::BLUE;
    pub const FUNCTION_BUILT_IN: Color = one_dark::CYAN;
    pub const KEYWORD: Color = one_dark::MAGENTA;
    pub const OPERATOR: Color = Color::WHITE;
    pub const PROPERTY: Color = Color::WHITE;
    pub const INTERPOLATION: Color = one_dark::LIGHT_YELLOW;
    pub const STRING: Color = one_dark::GREEN;
    pub const TYPE: Color = one_dark::WHITE;
    pub const VARIABLE: Color = one_dark::WHITE;
    pub const CONSTRUCTOR: Color = one_dark::BLUE;
    pub const CONSTANT: Color = one_dark::DARK_YELLOW;
    pub const LITERAL: Color = one_dark::DARK_YELLOW;
    pub const ESCAPE_SEQUENCE: Color = one_dark::CYAN;
    pub const COMMENT: Color = one_dark::COMMENT_GREY;

    pub const DEFAULT: Color = one_dark::WHITE;
}

pub mod blocks {
    use druid::Color;

    pub const CLASS: Color = Color::rgb8(230, 110, 54);
    pub const FUNCTION: Color = Color::rgb8(0, 120, 120);
    pub const IF: Color = Color::rgb8(128, 22, 56);
    pub const WHILE: Color = Color::rgb8(78, 0, 78);
    pub const FOR: Color = Color::rgb8(78, 0, 78);
    pub const TRY: Color = Color::rgb8(128, 51, 51);
    pub const COMMENT: Color = Color::rgb8(90, 90, 90);

    pub const GENERIC: Color = Color::rgb8(127, 51, 127);
}

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
