mod block_view;
mod parse;

use block_view::BlockView;
use iced::{Canvas, Element, Length, Sandbox, Settings, Scrollable, Color};
use std::{
    fs::File,
    io::{BufReader, Read},
};
use tree_sitter::Tree;

pub fn main() -> iced::Result {
    let mut settings = Settings::default();
    settings.default_font = Some(include_bytes!("../RobotoMono.ttf"));
    SitterDemo::run(settings)
}

fn get_test_string(name: &'static str) -> String {
    let file = File::open(name).expect("test file not found");
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader
        .read_to_string(&mut contents)
        .expect("could not read file");
    return contents;
}

struct SitterDemo {
    source: String,
    tree: Tree,
}

#[derive(Debug, Clone)]
enum Message {}

impl Sandbox for SitterDemo {
    type Message = Message;

    fn new() -> Self {
        let source = get_test_string("test.py");
        let tree = parse::parse(&source);

        SitterDemo { source, tree }
    }

    fn title(&self) -> String {
        String::from("Iced + Sitter Test")
    }

    fn update(&mut self, _message: Message) {
        /*
        would eventually
        1. take in an edit from the text input
        2. send an update to the parser object (that will eventually be in the parse file)
        3. set the tree in the view's state to what the object outputs
        */
    }

    fn view(&mut self) -> Element<Message> {
        let SitterDemo { source, tree } = self;

        // TODO: put inside scrollable
        // height and width might need to be hard set for that,
        // which might be able to be done using the (untested) commented code below
        let block = BlockView::new(source.clone(), tree.clone());
        Canvas::new(block)
            .width(Length::Fill) // might need to be hard set
            .height(Length::Fill)
            .into()
    }
}

//         // get height of text
//         let rows = self.source.lines().count();
//         // this might be a faster version, but I'm not sure about edge cases yet
//         // let rows = self.tree.root_node().end_position().row;

//         // get width of text
//         let columns = self.source.lines()
//             .map(|l| l.len())
//             .max()
//             .unwrap_or(0);

//         // return size of what will be drawn
//         layout::Node::new(Size::new(
//             (columns * FONT_WIDTH) as f32,
//             (rows * FONT_HEIGHT) as f32)
//         )
