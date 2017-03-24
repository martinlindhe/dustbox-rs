#![allow(dead_code)]

mod cpu;

mod tools;

fn main() {

    // XXX: /Users/m/dev/binary-samples/Executables/DOS-COM/
    let data = tools::read_binary("samples/vgafill/vgafill.com");

    let mut cpu = cpu::CPU::new();
    cpu.load_rom(&data, 0x100);

    for _ in 0..6 {
        cpu.execute_instruction();
        // XXX disasm of current pos...
        cpu.print_registers(); // XXX a repl loop: "r 1" == run 1 instruction, "r" == dump registers
    }
}
