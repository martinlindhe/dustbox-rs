#![allow(dead_code)]

mod cpu;
mod tools;

fn main() {

    // XXX: /Users/m/dev/binary-samples/Executables/DOS-COM/
    let data = tools::read_binary("samples/hellodos/hello.com");
    println!("{}", tools::to_hex_string(&data));

    let mut cpu = cpu::CPU::new();

    cpu.load_rom(&data);

    for _ in 0..5 {
        print!("{:04X}: ", cpu.pc);
        let disasm = cpu.disasm_instruction();
        println!("{}", disasm);
    }
}
