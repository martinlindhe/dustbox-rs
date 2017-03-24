#![allow(dead_code)]

//mod cpu;
mod disasm;

mod tools;

fn main() {

    // XXX: /Users/m/dev/binary-samples/Executables/DOS-COM/
    let data = tools::read_binary("samples/hellodos/hello.com");

    let mut disasm = disasm::Disassembly::new();
    println!("{}", disasm.disassemble(&data, 0x0100));
}
