use std::time::Instant;
use std::num::ParseIntError;
use std::io::Error as IoError;
use std::process::exit;

use cpu::CPU;
use register;
use register::{AX, BX, CX, DX, SI, DI, BP, SP, AL, CL, CS, DS, ES, FS, GS, SS};
use flags;
use tools;
use instruction::{seg_offs_as_flat, InstructionInfo};
use mmu::MMU;
use decoder::Decoder;
use segment::Segment;
use breakpoints::Breakpoints;

#[cfg(test)]
#[path = "./debugger_test.rs"]
mod debugger_test;

#[derive(Default)]
pub struct PrevRegs {
    pub ip: u16,
    pub r16: [register::Register16; 8], // general purpose registers
    pub sreg16: [u16; 6],               // segment registers
    pub flags: flags::Flags,
}

#[derive(Default)]
pub struct Debugger {
    pub cpu: CPU,
    pub prev_regs: PrevRegs,
    last_program: Option<String>,
    ip_breakpoints: Breakpoints, // break when IP reach address
    memory_breakpoints: Breakpoints, // break when memory change on this address
}

impl Debugger {
    pub fn new() -> Self {
        let mmu = MMU::new();
        let cpu = CPU::new(mmu);
        Debugger {
            cpu: cpu.clone(),
            prev_regs: PrevRegs {
                ip: cpu.ip,
                r16: cpu.r16,
                sreg16: cpu.sreg16,
                flags: cpu.flags,
            },
            last_program: None,
            ip_breakpoints: Breakpoints::new(),
            memory_breakpoints: Breakpoints::new(),
        }
    }

    pub fn is_ip_at_breakpoint(&self) -> bool {
        let offset = self.cpu.get_address();
        self.ip_breakpoints.hit(offset)
    }

    fn should_break(&self) -> bool {
        if self.cpu.fatal_error {
            return true;
        }
        if self.is_ip_at_breakpoint() {
            println!(
                "Breakpoint reached, ip = {:04X}:{:04X}",
                self.cpu.sreg16[CS],
                self.cpu.ip
            );
            return true;
        }
        // XXX was memory location changed?
        false
    }

    pub fn step_into(&mut self, cnt: usize) {
        let start = Instant::now();
        let mut done = 0;
        for _ in 0..cnt {
            self.cpu.execute_instruction();
            if self.should_break() {
                break;
            }
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
        let mut decoder = Decoder::new(self.cpu.mmu.clone());
        let op = decoder.disasm_instruction(self.cpu.sreg16[CS], self.cpu.ip);

        let dst_ip = self.cpu.ip + op.length as u16;
        println!("Step-over running to {:04X}", dst_ip);

        let mut cnt = 0;
        loop {
            cnt += 1;
            self.cpu.execute_instruction();
            if self.should_break() {
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

    fn run_until_breakpoint(&mut self) {
        loop {
            self.cpu.execute_instruction();
            if self.should_break() {
                break;
            }
        }
    }

    pub fn disasm_n_instructions_to_text(&mut self, n: usize) -> String {
        let mut rows: Vec<String> = Vec::new();
        let mut decoder = Decoder::new(self.cpu.mmu.clone());
        let ops = decoder.disassemble_block(
            self.cpu.sreg16[CS],
            self.cpu.ip,
            n as u16);

        for op in ops {
            rows.push(op.to_string());
        }
        rows.join("\n")
    }

    pub fn dump_memory(&self, filename: &str, base: usize, len: usize) -> Result<usize, IoError> {
        use std::path::Path;
        use std::fs::File;
        use std::io::Write;

        let path = Path::new(filename);
        let mut file = match File::create(&path) {
            Err(why) => return Err(why),
            Ok(file) => file,
        };
        let dump = self.cpu.mmu.dump_mem();

        if let Err(why) = file.write(&dump[base..base + len]) {
            return Err(why);
        }
        Ok(0)
    }

    pub fn exec_command(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        println!("> {}", cmd);
        let parts: Vec<String> = cmd.split(' ').map(|s| s.to_string()).collect();

         match parts[0].as_ref() {
            "help" => {
                println!("load <file>                      - load a binary (.com) file");
                println!("load                             - load previous binary (.com) file");
                println!("run                              - run until breakpoint");
                println!("step into <n>                    - steps into n instructions");
                println!("step over                        - steps over the next instruction");
                println!("reset                            - resets the cpu");
                println!("instcount                        - show number of instructions executed");
                println!("reg                              - show register values");
                println!("bp add <seg:off>                 - add breakpoint");
                println!("bp remove <seg:off>              - remove breakpoint");
                println!("bp list                          - show breakpoints");
                println!("bp clear                         - clear breakpoints");
                println!("flat                             - show current address as flat value");
                println!("disasm                           - disasm instruction");
                println!("hexdump <seg:off> <len>          - dumps len bytes of memory at given offset to the console");
                println!("bindump <seg:off> <len> <file>   - writes memory dump to file");
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
                        self.step_into(cnt);
                    },
                    "over" => {
                        self.step_over();
                    }
                     _ => {
                        println!("Unknown STEP sub-command: {}", cmd);
                    }
                }
            }
            "reset" => {
                println!("Resetting CPU");
                self.cpu.hard_reset(MMU::new());
            }
            "exit" | "quit" | "q" => {
                println!("Exiting ... {} instructions was executed",
                      self.cpu.instruction_count);
                exit(0);
            }
            "instcount" => {
                println!("Executed {} instructions", self.cpu.instruction_count);
            }
            "reg" | "regs" | "registers" => {
                self.print_registers();
            }
            "bp" | "breakpoint" => {
                if parts.len() < 2 {
                    println!("breakpoint: not enough arguments");
                } else {
                    match parts[1].as_ref() {
                        "help" => {
                            println!("Available breakpoint commands:");
                            println!("  bp add <seg:off>     add breakpoint");
                            println!("  bp remove <seg:off>  remove breakpoint");
                            println!("  bp clear             clears all breakpoints");
                            println!("  bp list              list all breakpoints");
                        }
                        "add" | "set" => {
                            match parse_segment_offset_pair(&parts[2]) {
                                Ok(bp) => {
                                    if self.ip_breakpoints.add(bp).is_some() {
                                        println!("Breakpoint added: {:06X}", bp);
                                    } else {
                                        println!("Breakpoint was already added");
                                    }
                                }
                                Err(e) => {
                                    println!("parse error: {:?}", e);
                                    return;
                                }
                            }
                        }
                        "del" | "delete" | "remove" => {
                            match parse_segment_offset_pair(&parts[2]) {
                                Ok(bp) => {
                                    match self.ip_breakpoints.remove(bp) {
                                        Some(_) => println!("Breakpoint removed: {:06X}", bp),
                                        None => println!("Breakpoint not found, so not removed!"),
                                    }
                                }
                                Err(e) => {
                                    println!("parse error: {:?}", e);
                                    return;
                                }
                            }
                        }
                        "clear" => {
                            self.ip_breakpoints.clear();
                        }
                        "list" => {
                            let list = self.ip_breakpoints.get();
                            let strs: Vec<String> =
                                list.iter().map(|b| format!("{:06X}", b)).collect();
                            let formatted_list = strs.join(" ");
                            println!("breakpoints: {}", formatted_list);
                        }
                        _ => println!("unknown breakpoint subcommand: {}", parts[1]),
                    }
                }
            }
            "flat" => {
                self.show_flat_address();
            }
            "d" | "disasm" => {
                let mut decoder = Decoder::new(self.cpu.mmu.clone());
                let op = decoder.disasm_instruction(
                    self.cpu.sreg16[CS],
                    self.cpu.ip
                );
                println!("{:?}", op);
                println!("{}", op);
            }
            "load" => {
                if parts.len() < 2 {
                    match self.last_program.clone() {
                        None        => println!("Filename not provided."),
                        Some(path)  => self.load_binary(&path),
                    }
                } else {
                    let path = parts[1..].join(" ").trim().to_string();
                    self.load_binary(&path);
                    self.last_program = Option::Some(path);
                }
            }
            "hexdump" => {
                // show dump of memory at <seg:off> <length>
                if parts.len() < 3 {
                    println!("hexdump: not enough arguments");
                    return;
                }

                let mem_dump = self.cpu.mmu.dump_mem();
                let mut pos: usize;
                let mut length: usize;

                match parse_segment_offset_pair(&parts[1]) {
                    Ok(p) => pos = p,
                    Err(e) => {
                        println!("parse error: {:?}", e);
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

                let mut row_cnt = 0;
                for i in pos..(pos + length) {
                    if row_cnt == 0 {
                        print!("[{:06X}] ", i);
                    }
                    print!("{:02X} ", mem_dump[i]);
                    row_cnt += 1;
                    if row_cnt == 16 {
                        println!();
                        row_cnt = 0;
                    }
                }
                println!();
            }
            "bindump" => {
                // bindump <seg:off> <len> <file>
                if parts.len() < 4 {
                    println!("bindump: not enough arguments");
                    return;
                }

                let mut pos: usize;
                let mut length: usize;

                match parse_segment_offset_pair(&parts[1]) {
                    Ok(p) => pos = p,
                    Err(e) => {
                        println!("parse error: {:?}", e);
                        return;
                    }
                }
                match parse_number_string(&parts[2]) {
                    Ok(n) => length = n,
                    Err(e) => {
                        println!("length parse error: {}", e);
                        return;
                    }
                }
                let filename = parts[3].trim();
                println!("Writing memory dump {}, len {:04X} to {}", parts[1], length, filename);
                if let Err(why) = self.dump_memory(filename, pos, length) {
                    println!("Dump memory failed: {}", why);
                }
            }
            "r" | "run" => {
                self.run_until_breakpoint();
            }
            "" => {}
            _ => {
                println!("Unknown command: {}", cmd);
            }
        }
    }

    pub fn load_binary(&mut self, name: &str) {
        println!("Reading raw binary from {}", name);
        match tools::read_binary(name) {
            Ok(data) => {
                self.cpu.hard_reset(MMU::new());
                self.cpu.load_com(&data);
            }
            Err(what) => println!("error {}", what),
        };
    }

    fn show_flat_address(&mut self) {
        let offset = self.cpu.get_address();
        let rom_offset = offset - self.cpu.get_rom_base() + 0x100;
        println!(
            "{:04X}:{:04X} is {:06X}.  rom offset is 0000:0100, or {:06X}",
            self.cpu.sreg16[CS],
            self.cpu.ip,
            offset,
            rom_offset
        );
    }

    fn print_registers(&mut self) -> String {
        let mut res = String::new();

        res += format!("AX:{:04X}  SI:{:04X}  DS:{:04X}  IP:{:04X}  cnt:{}\n",
                       self.cpu.r16[AX].val,
                       self.cpu.r16[SI].val,
                       self.cpu.sreg16[DS],
                       self.cpu.ip,
                       self.cpu.instruction_count)
                .as_ref();
        res += format!("BX:{:04X}  DI:{:04X}  CS:{:04X}  fl:{:04X}\n",
                       self.cpu.r16[BX].val,
                       self.cpu.r16[DI].val,
                       self.cpu.sreg16[CS],
                       self.cpu.flags.u16())
                .as_ref();
        res += format!("CX:{:04X}  BP:{:04X}  ES:{:04X}  GS:{:04X}\n",
                       self.cpu.r16[CX].val,
                       self.cpu.r16[BP].val,
                       self.cpu.sreg16[ES],
                       self.cpu.sreg16[GS])
                .as_ref();
        res += format!("DX:{:04X}  SP:{:04X}  FS:{:04X}  SS:{:04X}\n",
                       self.cpu.r16[DX].val,
                       self.cpu.r16[SP].val,
                       self.cpu.sreg16[FS],
                       self.cpu.sreg16[SS])
                .as_ref();
        res += format!("C{} Z{} S{} O{} A{} P{} D{} I{}",
                       self.cpu.flags.carry_numeric(),
                       self.cpu.flags.zero_numeric(),
                       self.cpu.flags.sign_numeric(),
                       self.cpu.flags.overflow_numeric(),
                       self.cpu.flags.auxiliary_numeric(),
                       self.cpu.flags.parity_numeric(),
                       self.cpu.flags.direction_numeric(),
                       self.cpu.flags.interrupt_numeric())
                .as_ref();

        res
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

// parses hex string to a integer
fn parse_hex_string(s: &str) -> Result<usize, ParseIntError> {
    let x = &s.replace("_", "");
    if x.len() >= 2 && &x[0..2] == "0x" {
        usize::from_str_radix(&x[2..], 16)
    } else {
        usize::from_str_radix(x, 16)
    }
}

// parses segment:offset pair to an integer
fn parse_segment_offset_pair(s: &str) -> Result<usize, ParseIntError> {
    let x = &s.replace("_", "");
    match x.find(':') {
        Some(pos) => {
            match parse_hex_string(&x[0..pos]) {
                Ok(segment) => {
                    match parse_hex_string(&x[pos+1..]) {
                        Ok(offset) => Ok(seg_offs_as_flat(segment as u16, offset as u16)),
                        Err(v) => Err(v),
                    }
                },
                Err(v) => Err(v),
            }
        }
        None => {
            // flat address
             match parse_hex_string(x) {
                Ok(val) => Ok(val),
                Err(v) => Err(v),
            }
        }
    }
}
