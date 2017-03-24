#![allow(dead_code)]

mod cpu;
mod disasm;

mod tools;

fn main() {

    // XXX: /Users/m/dev/binary-samples/Executables/DOS-COM/
    let data = tools::read_binary("samples/adrmode/adrmode.com");

    let mut cpu = cpu::CPU::new();
    cpu.load_rom(&data, 0x100);

    let mut disasm = disasm::Disassembly::new();
    disasm.load_rom(&data, 0x100);

    for _ in 0..3 {
        disasm.pc = cpu.pc;
        let op = disasm.disasm_instruction();
        println!("{:04X}: {}", op.offset, op.text);

        cpu.execute_instruction();
        cpu.print_registers(); // XXX a repl loop: "r 1" == run 1 instruction, "r" == dump registers
    }
}
