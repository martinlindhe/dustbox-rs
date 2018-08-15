extern crate dustbox;

use std::env;

use dustbox::machine::Machine;
use dustbox::tools;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("filename not supplied");
    }
    let filename = &args[1];

    println!("Opening {}", filename);

    let mut machine = Machine::default();
    machine.cpu.deterministic = true;
    match tools::read_binary(filename) {
        Ok(data) => machine.load_executable(&data),
        Err(err) => panic!("failed to read {}: {}", filename, err),
    }

    // XXX disasm flat, like in debugger
}
