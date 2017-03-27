#![allow(dead_code)]
#![allow(unused_attributes)]
#![allow(unused_imports)]
#[macro_use]

extern crate log;
extern crate colog;
// extern crate difference;

mod cpu;
mod disasm;

mod tools;


use std::fmt::Write;

fn main() {

    drop(colog::init());

    // XXX: /Users/m/dev/binary-samples/Executables/DOS-COM/
    //let app = "samples/adrmode/adrmode.com";
    //let app = "games/Beast (1984)(Dan Baker)/beast.com";
    let app = "samples/bar/bar.com";
    let data = tools::read_binary(app);

    let mut cpu = cpu::CPU::new();
    cpu.load_rom(&data, 0x100);

    let mut disasm = disasm::Disassembly::new();

    for _ in 0..340 {
        disasm.pc = cpu.pc as usize;
        let data = cpu.read_u8_slice(disasm.pc, 10);
        let text = disasm.disassemble(&data, disasm.pc);
        info!("{}", text);

        cpu.execute_instruction();
        cpu.print_registers(); // XXX a repl loop: "r 1" == run 1 instruction, "r" == dump registers
    }
}
