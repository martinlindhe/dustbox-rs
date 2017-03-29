#![allow(dead_code)]
#![allow(unused_attributes)]
#![allow(unused_imports)]
#[macro_use]
#[macro_use(assert_diff)]

extern crate log;
extern crate colog;
extern crate regex;
extern crate difference;

use std::io::{self, stdout, BufRead, Write};
use regex::Regex;
use std::process::exit;

mod cpu;
mod tools;

fn main() {

    drop(colog::init());

    // XXX: /Users/m/dev/binary-samples/Executables/DOS-COM/
    //let app = "samples/adrmode/adrmode.com";
    let app = "../dos-software-decoding/games/Blort (1987)(Hennsoft)/blort.com";
    //let app = "../dos-software-decoding/games/Dig Dug (1982)(Namco)/digdug.com";
    //let app = "samples/bar/bar.com";
    let data = tools::read_binary(app);

    let mut cpu = cpu::CPU::new();
    cpu.load_rom(&data, 0x100);

    let stdin = io::stdin();

    loop {
        let offset = cpu.get_offset();
        print!("{:06X}> ", offset);
        let _ = stdout().flush();

        let mut line = String::new();
        stdin.lock().read_line(&mut line).unwrap();

        let parts: Vec<String> = line.split(" ")
            .map(|s| s.trim_right().to_string())
            .collect();
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
                info!("{}", op.pretty_string());
            }
            "v" => {
                info!("Executed {} instructions", cpu.instruction_count);
            }
            "e" => {
                let n = if parts.len() < 2 {
                    1
                } else {
                    parts[1].parse::<usize>().unwrap()
                };

                info!("Executing {} instructions", n);
                for _ in 0..n {
                    let op = cpu.disasm_instruction();
                    info!("{}", op.pretty_string());
                    cpu.execute_instruction();
                }
            }
            "bp" | "breakpoint" => {
                // breakpoints
                // XXX: "bp remove 0x123"
                // XXX: "bp clear" = remove all breakpoints
                if parts.len() < 2 {
                    error!("breakpoint: not enough arguments");
                } else {
                    match parts[1].as_ref() {
                        "add" | "set" => {
                            let bp = parse_number_string(&parts[2]);
                            cpu.add_breakpoint(bp);
                            info!("Breakpoint added: {:04X}", bp);
                        }
                        "clear" => {
                            error!("XXX clear breakpoints");
                        }
                        "list" => {
                            error!("XXX LIST BREAKPOINTS");
                            let list = cpu.get_breakpoints(); // .sort();
                            // XXXX sort list

                            let strs: Vec<String> =
                                list.iter().map(|b| format!("{:04X}", b)).collect();
                            let formatted_list = strs.join(" ");
                            warn!("breakpoints: {}", formatted_list);
                        }
                        _ => error!("unknown breakpoint subcommand: {}", parts[1]),
                    }
                }
            }
            "run" => {
                let list = cpu.get_breakpoints();
                warn!("Executing until we hit a breakpoint");

                loop {
                    if cpu.execute_instruction() == false {
                        error!("Failed to execute instruction, breaking");
                        break;
                    }
                    let offset = cpu.ip as usize;

                    // XXX if op wasnt recognized, break

                    // if op.offset is in list, break
                    let mut list_iter = list.iter();
                    match list_iter.find(|&&x| x == offset) {
                        Some(n) => {
                            warn!("Breakpoint reached {:04X}", n);
                            break;
                        }
                        None => {}
                    }
                }
            }
            "exit" | "quit" | "q" => {
                info!("Exiting ... {} instructions was executed",
                      cpu.instruction_count);
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

fn parse_number_string(s: &str) -> usize {
    // XXX return Option, none = failed to parse
    if &s[0..2] == "0x" {
        let x = usize::from_str_radix(&s[2..], 16).unwrap();
        x
    } else {
        // decimal
        s.parse::<usize>().unwrap()
    }
}
