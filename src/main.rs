#![allow(dead_code)]
#![allow(unused_attributes)]
#![allow(unused_imports)]
#[macro_use]
#[macro_use(assert_diff)]

extern crate log;
extern crate colog;
extern crate regex;
extern crate difference;
extern crate time;

use std::io::{self, stdout, BufRead, Write};
use regex::Regex;
use std::process::exit;

mod cpu;
mod tools;

fn main() {

    drop(colog::init());

    // XXX: /Users/m/dev/binary-samples/Executables/DOS-COM/
    //let app = "samples/adrmode/adrmode.com";
    let games_root = "../dos-software-decoding/games".to_owned();
    //let app = games_root + "/8088 Othello (1985)(Bayley)/8088_othello.com";
    //let app = games_root + "/Apple Panic (1982)(Broderbund Software Inc)/panic.com";
    //let app = games_root + "/Astro Dodge (1982)(Digital Marketing Corporation)/astroids.com";
    let app = games_root + "/Beast (1984)(Dan Baker)/beast.com";
    //let app = games_root + "/Blort (1987)(Hennsoft)/blort.com";
    //let app = games_root + "/Dig Dug (1982)(Namco)/digdug.com";
    //let app = "samples/bar/bar.com";
    let data = tools::read_binary(&app);

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
                info!("{:?}", op);
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
                if parts.len() < 2 {
                    error!("breakpoint: not enough arguments");
                } else {
                    match parts[1].as_ref() {
                        "help" => {
                            info!("Available breakpoint commands:");
                            info!("  bp add 0x123     adds a breakpoint");
                            info!("  bp clear         clears all breakpoints");
                            info!("  bp list          list all breakpoints");
                        }
                        "add" | "set" => {
                            let bp = parse_number_string(&parts[2]);
                            cpu.add_breakpoint(bp);
                            info!("Breakpoint added: {:04X}", bp);
                        }
                        "clear" => {
                            cpu.clear_breakpoints();
                        }
                        "list" => {
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
