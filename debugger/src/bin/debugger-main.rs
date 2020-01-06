use std::rc::Rc;
use std::cell::RefCell;

use debugger::interface::Interface;
use dustbox::debug::Debugger;

fn main() {
    let app = Rc::new(RefCell::new(Debugger::default()));

    let mut gui = Interface::default(app);
    gui.main();
}
