use std::time::Instant;
use std::num::ParseIntError;

use cpu::CPU;
use register;
use register::CS;
use flags;
use tools;
use instruction::{seg_offs_as_flat, InstructionInfo};

#[cfg(test)]
#[path = "./debugger_test.rs"]
mod debugger_test;

pub struct PrevRegs {
    pub ip: u16,
    pub r16: [register::Register16; 8], // general purpose registers
    pub sreg16: [u16; 6],               // segment registers
    pub flags: flags::Flags,
}

pub struct Debugger {
    pub cpu: CPU,
    pub prev_regs: PrevRegs,
    last_program: Option<String>,
}

impl Debugger {
    pub fn new() -> Self {
        let cpu = CPU::new();
        Debugger {
            cpu: cpu.clone(),
            prev_regs: PrevRegs {
                ip: cpu.ip,
                r16: cpu.r16,
                sreg16: cpu.sreg16,
                flags: cpu.flags,
            },
            last_program: Option::None
        }
    }

    fn step_into(&mut self) {
        self.cpu.execute_instruction();

        if self.cpu.fatal_error {
            return;
        }

        if self.cpu.is_ip_at_breakpoint() {
            self.cpu.fatal_error = true;
            warn!(
                "Breakpoint reached (step-into), ip = {:04X}:{:04X}",
                self.cpu.sreg16[CS],
                self.cpu.ip
            );
            return;
        }
    }

    fn step_into_n_instructions(&mut self, cnt: usize) {
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
        let ms = (elapsed.as_secs() * 1_000) + u64::from(elapsed.subsec_nanos() / 1_000_000);
        println!(
            "Executed total {} instructions ({} now) in {} ms",
            self.cpu.instruction_count,
            done,
            ms
        );
    }

    pub fn step_over(&mut self) {
        if self.cpu.fatal_error {
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
        println!(
            "Step-over to {:04X} done, executed {} instructions",
            dst_ip,
            cnt
        );
    }

    pub fn disasm_n_instructions_to_text(&mut self, n: usize) -> String {
        let mut rows: Vec<String> = Vec::new();
        for op in self.disasm_n_instructions(n) {
            rows.push(op.to_string());
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

    pub fn dump_memory(&self, filename: &str, segment: u16, offset: u16, len: usize) {
        use std::path::Path;
        use std::fs::File;
        use std::io::Write;

        println!("Writing memory dump {:04X}:{:04X}, len {:04X} to {}", segment, offset, len, filename);

        let path = Path::new(filename);

        let mut file = match File::create(&path) {
            Err(why) => panic!("Failed to create {:?}: {}", path, why),
            Ok(file) => file,
        };

        let base = seg_offs_as_flat(segment, offset);
        if let Err(why) = file.write(&self.cpu.memory.memory[base..base + len]) {
            panic!("Failed to write to {:?}: {}", path, why);
        }
    }
    
    pub fn exec_command(&mut self, cmd: &str) {

        let parts: Vec<String> = cmd.trim().split(' ').map(|s| s.to_string()).collect();

         match parts[0].as_ref() {
            "help" => {
                println!("load <file>                      - load a binary (.com) file");
                println!("load                             - load previous binary (.com) file");
                println!("r                                - run until breakpoint");
                println!("step into <n>                    - steps into n instructions");
                println!("step over                        - steps over the next instruction");
                println!("reset                            - resets the cpu");
                println!("v                                - show number of instructions executed");
                println!("r                                - show register values");
                println!("bp add <n>                       - add a breakpoint at offset n");
                println!("bp list                          - show breakpoints");
                println!("bp clear                         - clear breakpoints");
                println!("flat                             - show current address as flat value");
                println!("d                                - disasm instruction");
                println!("hexdump <off> <len>              - dumps len bytes of memory at given offset to the console");
                println!("bindump <seg> <off> <len> <file> - writes memory dump to file");
                println!("exit                             - exit");
            }
            "step" => {
                match parts[1].as_ref() {
                    "into" => {
                        let mut cnt = 1;
                        if parts.len() > 2 {
                            match parse_number_string(&parts[2]) {
                                Ok(n) => cnt = n,
                                Err(e) => {
                                    println!("parse error: {}", e);
                                    return;
                                }
                            }
                        };
                        self.step_into_n_instructions(cnt);
                    },
                    "over" => {
                        // TODO: parse arg 3 (count)
                        self.step_over();
                    }
                     _ => {
                        println!("Unknown STEP sub-command: {}", cmd);
                    }
                }
            }
            "reset" => {
                println!("Resetting CPU");
                self.cpu.reset();
            }
            "exit" | "quit" | "q" => {
                use std::process::exit;

                info!("Exiting ... {} instructions was executed",
                      self.cpu.instruction_count);
                exit(0);
            }
            "v" => {
                info!("Executed {} instructions", self.cpu.instruction_count);
            }
            "reg" | "regs" | "registers" => {
                self.cpu.print_registers();
            }
            "bp" | "breakpoint" => {
                // breakpoints - all values are flat offsets
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
                            match parse_number_string(&parts[2]) {
                                Ok(bp) => {
                                    self.cpu.add_breakpoint(bp);
                                    info!("Breakpoint added: {:04X}", bp);
                                }
                                Err(e) => {
                                    println!("parse error: {}", e);
                                    return;
                                }
                            }
                        }
                        "del" | "delete" | "remove" => {
                            // TODO: "bp remove 0x123"
                            info!("TODO: remove breakpoint");
                        }
                        "clear" => {
                            self.cpu.clear_breakpoints();
                        }
                        "list" => {
                            let list = self.cpu.get_breakpoints(); // .sort();
                            // XXX sort list

                            let strs: Vec<String> =
                                list.iter().map(|b| format!("{:04X}", b)).collect();
                            let formatted_list = strs.join(" ");
                            warn!("breakpoints: {}", formatted_list);
                        }
                        _ => error!("unknown breakpoint subcommand: {}", parts[1]),
                    }
                }
            }
            "flat" => {
                self.show_flat_address();
            }
            "d" | "disasm" => {
                let op = self.cpu.disasm_instruction();
                info!("{:?}", op);
                info!("{}", op);
            }
            "load" => {
                if parts.len() < 2 {
                    match self.last_program.clone() {
                        None        => error!("Filename not provided."),
                        Some(path)  => self.load_binary(&path),
                    }
                } else {
                    let path = parts[1..].join(" ").trim().to_string();
                    self.load_binary(&path);
                    self.last_program = Option::Some(path);
                }
            }
            "hexdump" => {
                // dump memory at <offset> <length>
                let mut offset: usize = 0;
                let mut length: usize = 0xFFFF;
                if parts.len() >= 3 {
                    match parse_number_string(&parts[1]) {
                        Ok(n) => offset = n,
                        Err(e) => {
                            println!("parse error: {}", e);
                            return;
                        }
                    }
                    match parse_number_string(&parts[2]) {
                        Ok(n) => length = n,
                        Err(e) => {
                            println!("parse error: {}", e);
                            return;
                        }
                    }
                }
                for i in offset..(offset + length) {
                    print!("{:02X} ", self.cpu.memory.memory[i]);
                }
                println!();
            }
            "bindump" => {
                // bindump <seg> <off> <len> <file>
                if parts.len() < 5 {
                    println!("bindump: not enough arguments");
                    return;
                }

                let mut segment: usize;
                let mut offset: usize;
                let mut length: usize;
                match parse_number_string(&parts[1]) {
                    Ok(n) => segment = n,
                    Err(e) => {
                        println!("segment parse error: {}", e);
                        return;
                    }
                }
                match parse_number_string(&parts[2]) {
                    Ok(n) => offset = n,
                    Err(e) => {
                        println!("offset parse error: {}", e);
                        return;
                    }
                }
                match parse_number_string(&parts[3]) {
                    Ok(n) => length = n,
                    Err(e) => {
                        println!("length parse error: {}", e);
                        return;
                    }
                }
                self.dump_memory(parts[4].trim(), segment as u16, offset as u16, length);
            }
            "r" | "run" => {
                self.run_until_breakpoint();
            }
            "" => {}
            _ => {
                println!("Unknown command: {}", parts[0]);
            }
        }
    }

    pub fn load_binary(&mut self, name: &str) {
        info!("Reading rom from {}", name);
        match tools::read_binary(name) {
            Ok(data) => {
                self.cpu.reset();
                self.cpu.load_com(&data);
            }
            Err(what) => println!("error {}", what),
        };
    }

    fn show_flat_address(&mut self) {
        let offset = self.cpu.get_offset();
        let rom_offset = offset - self.cpu.get_rom_base() + 0x100;
        info!(
            "{:04X}:{:04X} is {:06X}.  rom offset is 0000:0100, or {:06X}",
            self.cpu.sreg16[CS],
            self.cpu.ip,
            offset,
            rom_offset
        );
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
}

// parses string to a integer. unprefixed values assume base 10, and "0x" prefix indicates base 16.
fn parse_number_string(s: &str) -> Result<usize, ParseIntError> {
    let x = &s.replace("_", "");
    if x.len() >= 2 && &x[0..2] == "0x" {
        // hex
        usize::from_str_radix(&x[2..], 16)
    } else {
        // decimal
        x.parse::<usize>()
    }
}
