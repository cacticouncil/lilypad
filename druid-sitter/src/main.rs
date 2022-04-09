mod block_widget;
mod parse;

use druid::widget::{EnvScope, TextBox};
use druid::{
    theme, AppLauncher, Color, Data, FontDescriptor, FontFamily, Insets, Key, Lens, PlatformError,
    Widget, WidgetExt, WindowDesc,
};
use parse::parse;
use std::rc::Rc;
use std::{
    fs::File,
    io::{BufReader, Read},
};
use tree_sitter::Tree;

const MONO_FONT: Key<FontDescriptor> = Key::new("org.cacticouncil.lilypad.mono-font");

fn main() -> Result<(), PlatformError> {
    // get data from test file
    let source = get_test_string("test.py");
    let tree = parse(&source);
    let data = Model {
        source,
        tree: Rc::new(tree),
    };

    // launch
    let main_window = WindowDesc::new(ui_builder).title("Druid + Sitter Test");
    AppLauncher::with_window(main_window)
        .configure_env(|env, _state| {
            env.set(
                MONO_FONT,
                FontDescriptor::new(FontFamily::new_unchecked("Roboto Mono")).with_size(15.0),
            );
        })
        .launch(data)
}

#[derive(Clone, Data, Lens)]
pub struct Model {
    source: String,
    tree: Rc<Tree>,
}

fn ui_builder() -> impl Widget<Model> {
    let blocks = block_widget::make_blocks();

    let editor = EnvScope::new(
        |env, _data| {
            // so the blocks can show through
            env.set(theme::BACKGROUND_LIGHT, Color::rgba8(0, 0, 0, 0));
            // so the blocks line up with the text
            env.set(theme::TEXTBOX_INSETS, Insets::uniform(0.0));
            // so the selection color is less aggressive
            env.set(theme::SELECTION_COLOR, Color::BLUE);
        },
        TextBox::multiline()
            .with_font(MONO_FONT)
            .lens(Model::source)
            .background(blocks),
    );

    editor
}

/* -------------------------------------------------------------------------- */
fn get_test_string(name: &'static str) -> String {
    let file = File::open(name).expect("test file not found");
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader
        .read_to_string(&mut contents)
        .expect("could not read file");
    return contents;
}
