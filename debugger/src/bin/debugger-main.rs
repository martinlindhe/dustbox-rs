extern crate dustbox_debugger;

use std::rc::Rc;
use std::cell::RefCell;

use dustbox_debugger::{debugger, interface};

fn main() {
    let app = Rc::new(RefCell::new(debugger::Debugger::default()));

    let mut gui = interface::Interface::default(app);
    gui.main();
}
