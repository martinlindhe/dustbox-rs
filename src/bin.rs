extern crate dustbox;

use std::rc::Rc;
use std::cell::RefCell;

use dustbox::{debugger, interface};

fn main() {
    let app = Rc::new(RefCell::new(debugger::Debugger::new()));

    let mut gui = interface::Interface::new(app);
    gui.main();
}
