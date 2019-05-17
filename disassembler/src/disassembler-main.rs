extern crate dustbox;
use dustbox::machine::Machine;
use dustbox::cpu::{Decoder};
use dustbox::debug::ProgramTracer;
use dustbox::tools;

extern crate clap;
use clap::{Arg, App};

fn main() {
    let matches = App::new("dustbox-disasm")
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
    println!();

    if matches.is_present("trace") {
        trace_disassembly(filename);
    } else {
        flat_disassembly(filename);
    }
}

fn flat_disassembly(filename: &str) {
    let mut machine = Machine::deterministic();
    match tools::read_binary(filename) {
        Ok(data) => machine.load_executable(&data),
        Err(err) => panic!("failed to read {}: {}", filename, err),
    }

    let mut decoder = Decoder::default();
    let mut ma = machine.cpu.get_memory_address();

    // XXX refactor, debugger has some form of disasm fn already
    loop {
        let op = decoder.get_instruction_info(&mut machine.mmu, ma.segment(), ma.offset());
        println!("{}", op);
        ma.inc_n(op.bytes.len() as u16);
        if ma.value() - u32::from(machine.rom_base.offset()) >= machine.rom_length as u32 {
            println!("XXX end of file. ma = {:04x}, rom_base = {:x}, rom_length = {:x}", ma.value(), machine.rom_base.offset(), machine.rom_length);
            break;
        }
    }
}

fn trace_disassembly(filename: &str) {
    let mut machine = Machine::deterministic();
    match tools::read_binary(filename) {
        Ok(data) => machine.load_executable(&data),
        Err(err) => panic!("failed to read {}: {}", filename, err),
    }
    let mut tracer = ProgramTracer::default();
    tracer.trace_execution(&mut machine);
    println!("{}", tracer.present_trace(&mut machine));
}
