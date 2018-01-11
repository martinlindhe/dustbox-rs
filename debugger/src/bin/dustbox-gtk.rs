extern crate dustbox_gtk;

use std::rc::Rc;
use std::cell::RefCell;

use dustbox_gtk::{debugger, interface};

fn main() {
    let app = Rc::new(RefCell::new(debugger::Debugger::new()));

    let mut gui = interface::Interface::new(app);
    gui.main();
}
