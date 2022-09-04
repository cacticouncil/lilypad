use slint::{SharedString, VecModel};
use std::rc::Rc;

mod background;
mod parse;

slint::include_modules!();

fn main() {
    let main_window = MainWindow::new();

    main_window.set_input(SharedString::from(include_str!("../test.py")));

    main_window.set_rectangles(get_rects(&main_window.get_input()).into());

    // set the callback for when the input is edited
    {
        let main_window_weak = main_window.as_weak();
        main_window.on_inputEdited(move || {
            let main_window = main_window_weak.unwrap();
            println!("input edited");
            main_window.set_rectangles(get_rects(&main_window.get_input()).into());
        });
    }

    main_window.run();
}

fn get_rects(source: &str) -> Rc<VecModel<BackgroundRect>> {
    let tree = parse::parse(source);
    let rects = background::rects_for_tree(&tree, source);
    Rc::new(VecModel::from(rects))
}
