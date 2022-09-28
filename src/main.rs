mod block_widget;
mod parse;

//use druid::widget::{EnvScope, TextBox};
use druid::widget::{Scroll};
use druid::{
    AppLauncher, Data, FontDescriptor, FontFamily, Key, Lens, PlatformError,
    Widget, WindowDesc,
};
use parse::{parse};
use std::rc::Rc;
use std::{
    fs::File,
    io::{BufReader, Read},
};
use tree_sitter::Tree;

const MONO_FONT: Key<FontDescriptor> = Key::new("org.cacticouncil.lilypad.mono-font");

fn main() -> Result<(), PlatformError> {
    // get data from test file
    let source = get_test_string("test1.py");
    let tree = parse(&source);
    let data = Model {
        source,
        tree: Rc::new(tree),
    };
    // launch
    let main_window = WindowDesc::new(ui_builder).title("Druid + Sitter Test"); //.window_size((1000.0, 1000.0))
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
    Scroll::new(block_widget::make_blocks()).vertical()
    //block_widget::make_blocks()
}

/* -------------------------------------------------------------------------- */
fn get_test_string(name: &'static str) -> String {
    let file = File::open(format!("{}{}", "test-files/", name)).expect("test file not found");
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader
        .read_to_string(&mut contents)
        .expect("could not read file");
    contents
}
