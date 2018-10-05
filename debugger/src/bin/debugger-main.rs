extern crate dustbox_debugger;
extern crate dustbox;

use std::rc::Rc;
use std::cell::RefCell;

use dustbox_debugger::interface::Interface;
use dustbox::debug::Debugger;

fn main() {
    let app = Rc::new(RefCell::new(Debugger::default()));

    let mut gui = Interface::default(app);
    gui.main();
}
