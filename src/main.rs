#![allow(dead_code)]

use std::fs::File;
use std::io::Read;
use std::process::exit;

mod cpu;


fn main() {

    // XXX: /Users/m/dev/binary-samples/Executables/DOS-COM/
    let data = read_binary("samples/hellodos/hello.com");
    println!("{}", to_hex_string(&data));

    let mut cpu = cpu::CPU::new();

    cpu.load_rom(&data);

    for _ in 0..5 {
        print!("{:04X}: ", cpu.pc);
        let disasm = cpu.disasm_instruction();
        println!("{}", disasm);
    }
}


pub fn read_binary(path: &str) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();

    let mut f = match File::open(path) {
        Ok(x) => x,
        Err(why) => {
            println!("Could not open file {}: {}", path, why);
            exit(1);
        }
    };

    match f.read_to_end(&mut buffer) {
        Ok(x) => x,
        Err(why) => {
            println!("could not read contents of file: {}", why);
            exit(1);
        }
    };

    buffer
}

pub fn to_hex_string(bytes: &Vec<u8>) -> String {
    let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    strs.join(" ")
}
