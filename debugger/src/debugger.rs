use std::time::Instant;
use std::num::ParseIntError;
use std::io::Error as IoError;
use std::process::exit;

use dustbox::machine::Machine;
use dustbox::cpu::register::{R16, SR, RegisterSnapshot};
use dustbox::cpu::decoder::Decoder;
use dustbox::tools;
use dustbox::memory::mmu::MemoryAddress;

use breakpoints::Breakpoints;
use memory_breakpoints::MemoryBreakpoints;

#[cfg(test)]
#[path = "./debugger_test.rs"]
mod debugger_test;

pub struct Debugger {
    pub machine: Machine,
    pub prev_regs: RegisterSnapshot,
    last_program: Option<String>,
    ip_breakpoints: Breakpoints, // break when IP reach address
    memory_breakpoints: MemoryBreakpoints, // break when memory change on this address
}

impl Debugger {
    pub fn new() -> Self {
        let machine = Machine::new();
        Debugger {
            prev_regs: machine.register_snapshot(),
            machine: machine,
            last_program: None,
            ip_breakpoints: Breakpoints::new(),
            memory_breakpoints: MemoryBreakpoints::new(),
        }
    }

    pub fn is_ip_at_breakpoint(&self) -> bool {
        let offset = self.machine.cpu.get_address();
        self.ip_breakpoints.hit(offset)
    }

    fn should_break(&mut self) -> bool {
        if self.machine.cpu.fatal_error {
            return true;
        }
        if self.is_ip_at_breakpoint() {
            println!(
                "Breakpoint reached, ip = {:04X}:{:04X}",
                self.machine.cpu.get_sr(&SR::CS),
                self.machine.cpu.ip
            );
            return true;
        }
        for addr in self.memory_breakpoints.get() {
            let val = self.machine.hw.mmu.memory.borrow().read_u8(addr);
            if self.memory_breakpoints.has_changed(addr, val) {
                println!("Value at memory breakpoint has changed. {:06X} = {:02X}", addr, val);
                return true;
            }
        }
        false
    }

    pub fn step_into(&mut self, cnt: usize) {
        let start = Instant::now();
        let mut done = 0;
        for _ in 0..cnt {
            self.machine.execute_instruction();
            if self.should_break() {
                break;
            }
            done += 1;
        }
        let elapsed = start.elapsed();
        let ms = (elapsed.as_secs() * 1_000) + u64::from(elapsed.subsec_nanos() / 1_000_000);
        println!(
            "Executed total {} instructions ({} now) in {} ms",
            self.machine.cpu.instruction_count,
            done,
            ms
        );
    }

    pub fn step_over(&mut self) {
        let mut decoder = Decoder::new();
        let op = decoder.decode_instruction(&mut self.machine.hw.mmu, self.machine.cpu.get_sr(&SR::CS), self.machine.cpu.ip);

        let dst_ip = self.machine.cpu.ip + op.bytes.len() as u16;
        println!("Step-over running to {:04X}", dst_ip);

        let mut cnt = 0;
        loop {
            cnt += 1;
            self.machine.execute_instruction();
            if self.should_break() {
                break;
            }
            if self.machine.cpu.ip == dst_ip {
                break;
            }
        }
        println!(
            "Step-over to {:04X} done, executed {} instructions ({} total)",
            dst_ip,
            cnt,
            self.machine.cpu.instruction_count
        );
    }

    pub fn disasm_n_instructions_to_text(&mut self, n: usize) -> String {
        let mut decoder = Decoder::new();
        decoder.disassemble_block_to_str(&mut self.machine.hw.mmu, self.machine.cpu.get_sr(&SR::CS), self.machine.cpu.ip, n)
    }

    pub fn dump_memory(&self, filename: &str, base: u32, len: u32) -> Result<usize, IoError> {
        use std::path::Path;
        use std::fs::File;
        use std::io::Write;

        let path = Path::new(filename);
        let mut file = match File::create(&path) {
            Err(why) => return Err(why),
            Ok(file) => file,
        };
        let dump = self.machine.hw.mmu.dump_mem();

        let base = base as usize;
        let len = len as usize;
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
                println!("membp add <seg:off>              - add memory breakpoint");
                println!("membp remove <seg:off>           - remove memory breakpoint");
                println!("membp list                       - show memory breakpoints");
                println!("membp clear                      - clear memory breakpoints");
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
                        self.step_into(cnt as usize);
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
                println!("Resetting machine");
                self.machine.hard_reset();
            }
            "exit" | "quit" | "q" => {
                println!("Exiting ... {} instructions was executed",
                      self.machine.cpu.instruction_count);
                exit(0);
            }
            "instcount" => {
                println!("Executed {} instructions", self.machine.cpu.instruction_count);
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
                            match self.parse_segment_offset_pair(&parts[2]) {
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
                            match self.parse_segment_offset_pair(&parts[2]) {
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
                            println!("Breakpoints: {}", formatted_list);
                        }
                        _ => println!("unknown breakpoint subcommand: {}", parts[1]),
                    }
                }
            }
            "membp" => {
                if parts.len() < 2 {
                    println!("memory breakpoint: not enough arguments");
                } else {
                    match parts[1].as_ref() {
                        "help" => {
                            println!("Available memory breakpoint commands:");
                            println!("  membp add <seg:off>     add breakpoint");
                            println!("  membp remove <seg:off>  remove breakpoint");
                            println!("  membp clear             clears all breakpoints");
                            println!("  membp list              list all breakpoints");
                        }
                        "add" | "set" => {
                            match self.parse_segment_offset_pair(&parts[2]) {
                                Ok(bp) => {
                                    if self.memory_breakpoints.add(bp).is_some() {
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
                            match self.parse_segment_offset_pair(&parts[2]) {
                                Ok(bp) => {
                                    match self.memory_breakpoints.remove(bp) {
                                        Some(_) => println!("Memory breakpoint removed: {:06X}", bp),
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
                            self.memory_breakpoints.clear();
                        }
                        "list" => {
                            let list = self.memory_breakpoints.get();
                            let strs: Vec<String> =
                                list.iter().map(|b| format!("{:06X}", b)).collect();
                            let formatted_list = strs.join(" ");
                            println!("Memory breakpoints: {}", formatted_list);
                        }
                        _ => println!("unknown breakpoint subcommand: {}", parts[1]),
                    }
                }
            }
            "flat" => {
                self.show_flat_address();
            }
            "d" | "disasm" => {
                let mut decoder = Decoder::new();
                let op = decoder.decode_instruction(&mut self.machine.hw.mmu, self.machine.cpu.get_sr(&SR::CS), self.machine.cpu.ip);
                println!("{:?}", op);
                println!("{}", op);
            }
            "load" => {
                if parts.len() < 2 {
                    match self.last_program.clone() {
                        None       => println!("Filename not provided."),
                        Some(path) => self.load_binary(&path),
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

                let mem_dump = self.machine.hw.mmu.dump_mem();
                let mut pos: u32;
                let mut length: u32;

                match self.parse_segment_offset_pair(&parts[1]) {
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
                    print!("{:02X} ", mem_dump[i as usize]);
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

                let mut pos: u32;
                let mut length: u32;

                match self.parse_segment_offset_pair(&parts[1]) {
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
                self.machine.execute_frame();
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
                self.machine.hard_reset();
                self.machine.load_com(&data);
            }
            Err(what) => println!("error {}", what),
        };
    }

    fn show_flat_address(&mut self) {
        let offset = self.machine.cpu.get_address();
        let rom_offset = offset - self.machine.cpu.get_rom_base() + 0x100;
        println!(
            "{:04X}:{:04X} is {:06X}.  rom offset is 0000:0100, or {:06X}",
            self.machine.cpu.get_sr(&SR::CS),
            self.machine.cpu.ip,
            offset,
            rom_offset
        );
    }

    // parses segment:offset pair to an integer
    fn parse_segment_offset_pair(&self, s: &str) -> Result<u32, ParseIntError> {
        let x = &s.replace("_", "");
        match x.find(':') {
            Some(pos) => {
                match self.parse_register_hex_string(&x[0..pos]) {
                    Ok(segment) => {
                        match self.parse_register_hex_string(&x[pos+1..]) {
                            Ok(offset) => Ok(MemoryAddress::RealSegmentOffset(segment as u16, offset as u16).value()),
                            Err(v) => Err(v),
                        }
                    },
                    Err(v) => Err(v),
                }
            }
            None => {
                // flat address
                match self.parse_register_hex_string(x) {
                    Ok(val) => Ok(val as u32),
                    Err(v) => Err(v),
                }
            }
        }
    }

    // parses hex string or register name to a integer
    fn parse_register_hex_string(&self, s: &str) -> Result<usize, ParseIntError> {
        let x = &s.replace("_", "");
        let x = x.to_lowercase();
        if x.len() >= 2 && &x[0..2] == "0x" {
            usize::from_str_radix(&x[2..], 16)
        } else {
            match x.as_ref() {
                "ax" => Ok(self.machine.cpu.get_r16(&R16::AX) as usize),
                "bx" => Ok(self.machine.cpu.get_r16(&R16::BX) as usize),
                "cx" => Ok(self.machine.cpu.get_r16(&R16::CX) as usize),
                "dx" => Ok(self.machine.cpu.get_r16(&R16::DX) as usize),
                "sp" => Ok(self.machine.cpu.get_r16(&R16::SP) as usize),
                "bp" => Ok(self.machine.cpu.get_r16(&R16::BP) as usize),
                "si" => Ok(self.machine.cpu.get_r16(&R16::SI) as usize),
                "di" => Ok(self.machine.cpu.get_r16(&R16::DI) as usize),
                "es" => Ok(self.machine.cpu.get_sr(&SR::ES) as usize),
                "cs" => Ok(self.machine.cpu.get_sr(&SR::CS) as usize),
                "ss" => Ok(self.machine.cpu.get_sr(&SR::SS) as usize),
                "ds" => Ok(self.machine.cpu.get_sr(&SR::DS) as usize),
                "fs" => Ok(self.machine.cpu.get_sr(&SR::FS) as usize),
                "gs" => Ok(self.machine.cpu.get_sr(&SR::GS) as usize),
                _ => usize::from_str_radix(&x, 16)
            }
        }
    }

    fn print_registers(&mut self) -> String {
        let mut res = String::new();

        res += format!("AX:{:04X}  SI:{:04X}  DS:{:04X}  IP:{:04X}  cnt:{}\n",
                       self.machine.cpu.get_r16(&R16::AX),
                       self.machine.cpu.get_r16(&R16::SI),
                       self.machine.cpu.get_sr(&SR::DS),
                       self.machine.cpu.ip,
                       self.machine.cpu.instruction_count)
                .as_ref();
        res += format!("BX:{:04X}  DI:{:04X}  CS:{:04X}  fl:{:04X}\n",
                       self.machine.cpu.get_r16(&R16::BX),
                       self.machine.cpu.get_r16(&R16::DI),
                       self.machine.cpu.get_sr(&SR::CS),
                       self.machine.cpu.flags.u16())
                .as_ref();
        res += format!("CX:{:04X}  BP:{:04X}  ES:{:04X}  GS:{:04X}\n",
                       self.machine.cpu.get_r16(&R16::CX),
                       self.machine.cpu.get_r16(&R16::BP),
                       self.machine.cpu.get_sr(&SR::ES),
                       self.machine.cpu.get_sr(&SR::GS))
                .as_ref();
        res += format!("DX:{:04X}  SP:{:04X}  FS:{:04X}  SS:{:04X}\n",
                       self.machine.cpu.get_r16(&R16::DX),
                       self.machine.cpu.get_r16(&R16::SP),
                       self.machine.cpu.get_sr(&SR::FS),
                       self.machine.cpu.get_sr(&SR::SS))
                .as_ref();
        res += format!("C{} Z{} S{} O{} A{} P{} D{} I{}",
                       self.machine.cpu.flags.carry_numeric(),
                       self.machine.cpu.flags.zero_numeric(),
                       self.machine.cpu.flags.sign_numeric(),
                       self.machine.cpu.flags.overflow_numeric(),
                       self.machine.cpu.flags.adjust_numeric(),
                       self.machine.cpu.flags.parity_numeric(),
                       self.machine.cpu.flags.direction_numeric(),
                       self.machine.cpu.flags.interrupt_numeric())
                .as_ref();

        res
    }
}

// parses string to a integer. unprefixed values assume base 10, and "0x" prefix indicates base 16.
fn parse_number_string(s: &str) -> Result<u32, ParseIntError> {
    let x = &s.replace("_", "");
    if x.len() >= 2 && &x[0..2] == "0x" {
        // hex
        u32::from_str_radix(&x[2..], 16)
    } else {
        // decimal
        x.parse::<u32>()
    }
}
