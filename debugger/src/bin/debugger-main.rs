use std::rc::Rc;
use std::cell::RefCell;

use clap::{Arg, App};

use debugger::interface::Interface;
use dustbox::debug::Debugger;

fn main() {
    let matches = App::new("dustbox-disasm")
            .version("0.1")
            .arg(Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .index(1))
            .get_matches();

    let mut debugger = Debugger::default();

    if matches.is_present("INPUT") {
        let filename = matches.value_of("INPUT").unwrap();
        debugger.load_executable(&filename);
    }

    let app = Rc::new(RefCell::new(debugger));

    let mut gui = Interface::default(app);
    gui.main();
}
