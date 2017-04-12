use std::io::{self, stdout, BufRead, Write};
use regex::Regex;
use std::process::exit;

use cpu;
use tools;

pub struct Debugger {
    cpu: cpu::CPU,
    stdin: io::Stdin,
    stdout: io::Stdout,
}

pub fn new() -> Debugger {
    Debugger {
        cpu: cpu::CPU::new(),
        stdin: io::stdin(),
        stdout: io::stdout(),
    }
}

impl Debugger {
    pub fn start(&mut self) {
        //let bios = tools::read_binary("../dos-software-decoding/ibm-pc/ibm5550/ipl5550.rom");
        //self.cpu.load_bios(&bios);

        loop {
            self.prompt();
        }
    }

    fn prompt(&mut self) {
        print!("{:04X}:{:04X}> ", self.cpu.sreg16[cpu::CS], self.cpu.ip);
        let _ = self.stdout.flush();

        let parts = self.read_line();

        match parts[0].as_ref() {
            "load" => {
                if parts.len() < 2 {
                    error!("Filename not provided.");
                } else {
                    self.load_binary(parts[1..].join(" ").as_ref());
                }
            }
            "flat" => {
                self.show_flat_address();
            }
            "reset" => {
                info!("Resetting CPU");
                self.cpu.reset();
            }
            "r" | "reg" | "regs" => {
                self.cpu.print_registers();
            }
            "d" | "disasm" => {
                let op = self.cpu.disasm_instruction();
                info!("{:?}", op);
                info!("{}", op.pretty_string());
            }
            "v" => {
                info!("Executed {} instructions", self.cpu.instruction_count);
            }
            "e" => {
                let n = if parts.len() < 2 {
                    1
                } else {
                    parts[1].parse::<usize>().unwrap()
                };
                self.execute_n_instructions(n);
            }
            "bp" | "breakpoint" => {
                // breakpoints - all values are flat offsets
                // XXX: "bp remove 0x123"
                // XXX allow to enter bp in format "segment:offset"
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
                            self.cpu.add_breakpoint(bp);
                            info!("Breakpoint added: {:04X}", bp);
                        }
                        "clear" => {
                            self.cpu.clear_breakpoints();
                        }
                        "list" => {
                            let list = self.cpu.get_breakpoints(); // .sort();
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
                self.run();
            }
            "exit" | "quit" | "q" => {
                info!("Exiting ... {} instructions was executed",
                      self.cpu.instruction_count);
                exit(0);
            }
            "" => {}
            _ => {
                println!("Unknown command: {}", parts[0]);
            }
        }
    }

    fn load_binary(&mut self, name: &str) {
        let data = tools::read_binary(name);
        self.cpu.load_com(&data);
    }

    fn show_flat_address(&mut self) {
        let offset = self.cpu.get_offset();
        let rom_offset = offset - self.cpu.get_rom_base() + 0x100;
        info!("{:04X}:{:04X} is {:06X}.  rom offset is 0000:0100, or {:06X}",
              self.cpu.sreg16[cpu::CS],
              self.cpu.ip,
              offset,
              rom_offset);
    }

    fn execute_n_instructions(&mut self, n: usize) {
        info!("Executing {} instructions", n);
        for _ in 0..n {
            let op = self.cpu.disasm_instruction();
            info!("{}", op.pretty_string());
            self.cpu.execute_instruction();
        }
    }

    fn run(&mut self) {
        let list = self.cpu.get_breakpoints();
        warn!("Executing until we hit a breakpoint");

        loop {
            self.cpu.execute_instruction();
            if self.cpu.fatal_error {
                error!("Failed to execute instruction, breaking.");
                break;
            }
            let offset = self.cpu.get_offset();

            // break if we hit a breakpoint
            let mut list_iter = list.iter();
            if let Some(n) = list_iter.find(|&&x| x == offset) {
                warn!("Breakpoint reached {:04X}", n);
                break;
            }
        }
    }

    fn read_line(&mut self) -> Vec<String> {
        let mut line = String::new();
        self.stdin.lock().read_line(&mut line).unwrap();
        line.split(' ')
            .map(|s| s.trim_right().to_string())
            .collect()
    }
}

fn parse_number_string(s: &str) -> usize {
    // XXX return Option, none = failed to parse
    if s[0..2] == *"0x" {
        usize::from_str_radix(&s[2..], 16).unwrap()
    } else {
        // decimal
        s.parse::<usize>().unwrap()
    }
}
