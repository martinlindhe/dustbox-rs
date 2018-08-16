extern crate dustbox;
use dustbox::machine::Machine;
use dustbox::cpu::{Decoder, R};
use dustbox::memory::MemoryAddress;
use dustbox::tools;

extern crate clap;
use clap::{Arg, App};

mod tracer;

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
    println!("Opening {}", filename);

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
    let mut ma = MemoryAddress::RealSegmentOffset(machine.cpu.get_r16(R::CS), machine.cpu.regs.ip);

    loop {
        let op = decoder.get_instruction_info(&mut machine.hw.mmu, ma.segment(), ma.offset());
        println!("{}", op);
        ma.inc_n(op.bytes.len() as u16);
        if ma.value() - machine.cpu.rom_base >= machine.cpu.rom_length {
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
}
