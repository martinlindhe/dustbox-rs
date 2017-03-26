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
    disasm.load_rom(&data, 0x100);

    for _ in 0..340 {
        disasm.pc = cpu.pc;
        let op = disasm.disasm_instruction();
        info!("{:04X}: {}", op.offset, op.text);

        cpu.execute_instruction();
        cpu.print_registers(); // XXX a repl loop: "r 1" == run 1 instruction, "r" == dump registers
    }
}
