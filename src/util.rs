use druid::{
    piet::{PietTextLayout, Text, TextLayoutBuilder},
    text::FontFamily,
    PaintCtx,
};

use crate::theme;

pub fn make_label_text_layout(text: &str, ctx: &mut PaintCtx) -> PietTextLayout {
    let font_family = if cfg!(target_os = "macos") {
        FontFamily::new_unchecked("SF Pro Text")
    } else {
        FontFamily::new_unchecked("Helvetica")
    };

    ctx.text()
        .new_text_layout(text.to_string())
        .font(font_family, 15.0)
        .text_color(theme::INTERFACE_TEXT)
        .build()
        .unwrap()
}

pub fn rand_u64() -> u64 {
    let mut buf = [0u8; 8];
    getrandom::getrandom(&mut buf).expect("Failed to generate random bytes");
    u64::from_le_bytes(buf)
}
