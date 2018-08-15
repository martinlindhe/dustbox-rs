extern crate dustbox;

use std::env;

use dustbox::machine::Machine;
use dustbox::cpu::{Decoder, R};
use dustbox::memory::MemoryAddress;
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
