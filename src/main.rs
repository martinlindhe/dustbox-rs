#![allow(dead_code)]
#![allow(unused_attributes)]
#![allow(unused_imports)]
#[macro_use]

extern crate log;
extern crate colog;
extern crate regex;
extern crate difference;


mod cpu;
mod disasm;
mod tools;
mod opcodes;


use std::io::{self, stdout, BufRead, Write};
use regex::Regex;
use std::process::exit;

fn main() {

    drop(colog::init());

    // XXX: /Users/m/dev/binary-samples/Executables/DOS-COM/
    //let app = "samples/adrmode/adrmode.com";
    //let app = "games/Beast (1984)(Dan Baker)/beast.com";
    let app = "samples/bar/bar.com";
    let data = tools::read_binary(app);

    let mut cpu = cpu::CPU::new();
    cpu.load_rom(&data, 0x100);

    let stdin = io::stdin();

    loop {
        print!("{:06X}> ", cpu.ip);
        let _ = stdout().flush();

        let mut line = String::new();
        stdin.lock().read_line(&mut line).unwrap();

        let parts: Vec<String> = line.split(" ").map(|s| s.trim_right().to_string()).collect();
        match parts[0].as_ref() {
            "reset" => {
                info!("Resetting CPU");
                cpu.reset();
            }
            "r" | "reg" | "regs" => {
                cpu.print_registers();
            }
            "d" | "disasm" => {
                let op = cpu.disasm_instruction();
                warn!("XXX disasm WIP: {:?}", op);
            }
            "e" => {
                let n = if parts.len() < 2 {
                    1
                } else {
                    parts[1].parse::<i32>().unwrap()
                };

                info!("Executing {} instructions", n);
                for _ in 0..n {
                    cpu.execute_instruction();
                }

            }
            "exit" | "quit" | "q" => {
                info!("Exiting ...");
                exit(0);
            }
            "" => {}
            _ => {
                println!("Unknown command: {}", parts[0]);
            }
        }
    }
    /*

    let mut disasm = disasm::Disassembly::new();

    for _ in 0..340 {
        let pc = cpu.pc as usize;
        disasm.pc = pc;
        let data = cpu.read_u8_slice(pc, 10);
        let text = disasm.disassemble(&data, pc);
        info!("{}", text);

        cpu.execute_instruction();
        cpu.print_registers();
    }
*/
}
