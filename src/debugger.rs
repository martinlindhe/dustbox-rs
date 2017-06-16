use std::time::Instant;

use cpu::CPU;
use register::CS;
use tools;
use instruction::{InstructionInfo};

pub struct Debugger {
    pub cpu: CPU,
}

impl Debugger {
    pub fn new() -> Self {
        let mut dbg = Debugger { cpu: CPU::new() };
        // XXX for quick testing while building the ui
        // let name = "../dos-software-decoding/samples/bar/bar.com";
        let name = "../dos-software-decoding/demo-256/beziesux/beziesux.com";
        dbg.load_binary(name);
        //dbg.cpu.add_breakpoint(seg_offs_as_flat(0x085F, 0x0108)); 
        //dbg.cpu.add_breakpoint(seg_offs_as_flat(0x085F, 0x017D));
        dbg
    }

    /*
    pub fn start(&mut self) {
        //let bios = tools::read_binary("../dos-software-decoding/ibm-pc/ibm5550/ipl5550.rom");
        //self.cpu.load_bios(&bios);

        loop {
            self.prompt();
        }
    }
    */

    pub fn step_into(&mut self)  {
        self.cpu.execute_instruction();

        if self.cpu.fatal_error {
            // println!("XXX fatal error, not executing more");
            return;
        }

        if self.cpu.is_ip_at_breakpoint() {
            self.cpu.fatal_error = true;
            warn!("Breakpoint reached (step-into), ip = {:04X}:{:04X}",
                self.cpu.sreg16[CS],
                self.cpu.ip);
            return;
        }
    }

    pub fn step_into_n_instructions(&mut self, cnt: usize) {
        // measure time
        let start = Instant::now();
        let mut done = 0;
        for _ in 0..cnt {
            if self.cpu.fatal_error {
                break;
            }
            self.step_into();
            done += 1;
        }
        let elapsed = start.elapsed();
        let ms = (elapsed.as_secs() * 1_000) + (elapsed.subsec_nanos() / 1_000_000) as u64;
        println!("Executed total {} instructions ({} now) in {} ms", self.cpu.instruction_count, done, ms);
    }

    pub fn step_over(&mut self) {
        if self.cpu.fatal_error {
            // println!("XXX fatal error, not executing more");
            return;
        }
        let op = self.cpu.disasm_instruction();

        let dst_ip = self.cpu.ip + op.length as u16;
        println!("Step-over running to {:04X}", dst_ip);

        let mut cnt = 0;
        loop {
            cnt += 1;
            self.cpu.execute_instruction();

            if self.cpu.is_ip_at_breakpoint() {
                warn!("Breakpoint reached, breaking step-over");
                break;
            }

            if self.cpu.ip == dst_ip {
                break;
            }
        }
        println!("Step-over to {:04X} done, executed {} instructions",
                 dst_ip,
                 cnt);
    }

    pub fn disasm_n_instructions_to_text(&mut self, n: usize) -> String {
        let mut rows: Vec<String> = Vec::new();
        for op in self.disasm_n_instructions(n) {
            let s = op.pretty_string();
            rows.push(s);
        }
        rows.join("\n")
    }

    fn disasm_n_instructions(&mut self, n: usize) -> Vec<InstructionInfo> {
        let mut res: Vec<InstructionInfo> = Vec::new();
        let org_ip = self.cpu.ip;
        for _ in 0..n {
            let op = self.cpu.disasm_instruction();
            self.cpu.ip += op.length as u16;
            res.push(op);
        }
        self.cpu.ip = org_ip;
        res
    }

    /*
    fn prompt(&mut self) {
        print!("{:04X}:{:04X}> ", self.cpu.sreg16[CS], self.cpu.ip);
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
            "dump" => {
                // XXX dump memory at <offset> <length>
                if parts.len() < 3 {
                    error!("Syntax error: <offset> <length>");
                } else {
                    let offset = parse_number_string(&parts[1]);
                    let length = parse_number_string(&parts[2]);
                    for i in offset..(offset + length) {
                        print!("{:02X} ", self.cpu.memory.memory[i]);
                    }
                    println!("");
                }
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
    */

    pub fn load_binary(&mut self, name: &str) {
        let data = tools::read_binary(name);
        self.cpu.load_com(&data);
    }

    fn show_flat_address(&mut self) {
        let offset = self.cpu.get_offset();
        let rom_offset = offset - self.cpu.get_rom_base() + 0x100;
        info!("{:04X}:{:04X} is {:06X}.  rom offset is 0000:0100, or {:06X}",
              self.cpu.sreg16[CS],
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

    fn run_until_breakpoint(&mut self) {
        warn!("Executing until we hit a breakpoint");

        loop {
            self.cpu.execute_instruction();
            if self.cpu.fatal_error {
                error!("Failed to execute instruction, breaking.");
                break;
            }
            if self.cpu.is_ip_at_breakpoint() {
                self.cpu.fatal_error = true;
                 warn!("Breakpoint reached");
                break;
            }
        }
    }

    /*
    fn read_line(&mut self) -> Vec<String> {
        let mut line = String::new();
        self.stdin.lock().read_line(&mut line).unwrap();
        line.split(' ')
            .map(|s| s.trim_right().to_string())
            .collect()
    }
    */
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

