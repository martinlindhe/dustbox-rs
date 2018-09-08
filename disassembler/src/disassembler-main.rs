extern crate dustbox;
use dustbox::machine::Machine;
use dustbox::cpu::Decoder;
use dustbox::tools;

extern crate clap;
use clap::{Arg, App};

mod tracer;

#[cfg(test)] #[macro_use]
extern crate pretty_assertions;

fn main() {
    let matches = App::new("disassembler_dustbox")
            .version("0.1")
            .arg(Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(true)
                .index(1))
            .arg(Arg::with_name("trace")
                .long("trace")
                .help("Trace jump destinations while disassembling"))
            .get_matches();

    let filename = matches.value_of("INPUT").unwrap();
    println!("# Input file {}", filename);
    println!("");

    if matches.is_present("trace") {
        trace_disassembly(filename);
    } else {
        flat_disassembly(filename);
    }
}

fn flat_disassembly(filename: &str) {
    let mut machine = Machine::default();
    machine.cpu.deterministic = true;
    match tools::read_binary(filename) {
        Ok(data) => machine.load_executable(&data),
        Err(err) => panic!("failed to read {}: {}", filename, err),
    }

    let mut decoder = Decoder::default();
    let mut ma = machine.cpu.get_memory_address();

    loop {
        let op = decoder.get_instruction_info(&mut machine.hw.mmu, ma.segment(), ma.offset());
        println!("{}", op);
        ma.inc_n(op.bytes.len() as u16);
        if ma.value() - machine.rom_base.offset() as u32 >= machine.rom_length as u32 {
            break;
        }
    }
}

fn trace_disassembly(filename: &str) {
    let mut machine = Machine::default();
    machine.cpu.deterministic = true;
    match tools::read_binary(filename) {
        Ok(data) => machine.load_executable(&data),
        Err(err) => panic!("failed to read {}: {}", filename, err),
    }
    let mut tracer = tracer::Tracer::new();
    tracer.trace_execution(&mut machine);
    println!("{}", tracer.present_trace(&mut machine));
}
