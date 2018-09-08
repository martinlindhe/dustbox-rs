use std::cmp;

use dustbox::machine::Machine;
use dustbox::cpu::{Decoder, R, Op, Parameter};
use dustbox::memory::MemoryAddress;
use dustbox::string::right_pad;
use dustbox::hex::hex_bytes;

#[cfg(test)]
#[path = "./tracer_test.rs"]
mod tracer_test;

const DEBUG_TRACER: bool = false;

struct SeenDestination {
    /// segment:offset converted into real flat addresses
    address: MemoryAddress,

    sources: Vec<MemoryAddress>,

    visited: bool,
}

pub struct Tracer {
    seen_destinations: Vec<SeenDestination>,

    /// flat addresses of start of each visited opcode
    visited_addresses: Vec<MemoryAddress>,

    /// finalized analysis result
    accounted_bytes: Vec<GuessedDataAddress>,
}

#[derive(Clone, Eq, PartialEq)]
enum GuessedDataType {
    InstrStart,
    InstrContinuation,
    UnknownByte,
}

#[derive(Eq, PartialEq)]
struct GuessedDataAddress {
    kind: GuessedDataType,
    address: MemoryAddress,
}


impl PartialOrd for GuessedDataAddress {
    fn partial_cmp(&self, other: &GuessedDataAddress) -> Option<cmp::Ordering> {
        Some(other.cmp(self))
    }
}

impl Ord for GuessedDataAddress {
    fn cmp(&self, other: &GuessedDataAddress) -> cmp::Ordering {
        other.address.value().cmp(&self.address.value())
    }
}

impl Tracer {
    pub fn new() -> Self {
        Tracer {
            seen_destinations: Vec::new(),
            visited_addresses: Vec::new(),
            accounted_bytes: Vec::new(),
        }
    }

    pub fn trace_execution(&mut self, machine: &mut Machine) {
        // tell tracer to start at CS:IP
        let ma = MemoryAddress::RealSegmentOffset(machine.cpu.get_r16(R::CS), machine.cpu.regs.ip);
        self.seen_destinations.push(SeenDestination{address: ma, visited: false, sources: Vec::new()});

        loop {
            self.trace_unvisited_destination(machine);
            if !self.has_any_unvisited_destinations() {
                if DEBUG_TRACER {
                    println!("exhausted all destinations, breaking!");
                }
                break;
            }
        }

        self.post_process_execution(machine);
    }

    fn post_process_execution(&mut self, machine: &mut Machine) {
        // walk each byte of the loaded rom and check w instr lengths
        // if any bytes are not known to occupy, allows for us to show them as data
        let mut decoder = Decoder::default();

        for ma in &self.visited_addresses {
            // translate address into physical offset
            let abs = (ma.value() - machine.rom_base.offset() as u32) as usize;

            let ii = decoder.get_instruction_info(&mut machine.hw.mmu, ma.segment(), ma.offset());

            let mut adr = ma.clone();
            self.accounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::InstrStart, address: adr});
            if  DEBUG_TRACER {
                println!("add start instr at {}", adr);
            }
            for _ in abs + 1..(abs + ii.instruction.length as usize) {
                adr.inc_u8();
                self.accounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::InstrContinuation, address: adr.clone()});
                if  DEBUG_TRACER {
                    println!("add continuation instr at {}", adr);
                }
            }
        }

        // xxx find all unvisited offsets
        let mut unaccounted_bytes = vec![];
        for ofs in (machine.rom_base.offset() as usize)..(machine.rom_base.offset() as usize + machine.rom_length) {
            let adr = MemoryAddress::RealSegmentOffset(machine.rom_base.segment(), ofs as u16);

            let mut found = false;
            for ab in &self.accounted_bytes {
                if ab.address == adr {
                    found = true;
                    break;
                }
            }
            if !found {
                if  DEBUG_TRACER {
                    println!("address is unaccounted {}", adr);
                }
                unaccounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::UnknownByte, address: adr});
            }
        }

        for ub in unaccounted_bytes {
            self.accounted_bytes.push(ub);
        }

        // XXX sort accounted bytes
        self.accounted_bytes.sort();
    }

    /// presents a flatish traced disassembly
    pub fn present_trace(&mut self, machine: &mut Machine) -> String {

        // Displays decoded instructions at the known instruction offsets
        let mut decoder = Decoder::default();
        let mut res = String::new();

        for ab in &self.accounted_bytes {
            match ab.kind {
                GuessedDataType::InstrStart => {
                    let ii = decoder.get_instruction_info(&mut machine.hw.mmu, ab.address.segment(), ab.address.offset());
                    let xref = self.render_xref(&ab.address);
                    if xref != "" {
                        let ins = format!("{}", ii);
                        res.push_str(&format!("{}{}", right_pad(&ins, 68), xref));
                    } else {
                        res.push_str(&format!("{}", ii));
                    }
                    res.push('\n');
                }
                GuessedDataType::InstrContinuation => {},
                GuessedDataType::UnknownByte => {
                    let ii = decoder.get_instruction_info(&mut machine.hw.mmu, ab.address.segment(), ab.address.offset());
                    res.push_str(&format!("[{}] {}               db       0x{:02x}", ab.address, hex_bytes(&ii.bytes), ii.bytes[0]));
                    res.push('\n');
                }
            }
        }

        res
    }

    /// show branch cross references
    fn render_xref(&self, ma: &MemoryAddress) -> String {
        let mut s = String::new();
        if let Some(mut sources) = self.get_sources_for_destination(*ma) {
            sources.sort();
            let mut source_offsets = Vec::new();
            for src in &sources {
                source_offsets.push(format!("{:04X}:{:04X}", src.segment(), src.offset()));
            }
            s = format!("; xref: {}", source_offsets.join(", "));
        }

        s
    }

    fn learn_destination(&mut self, seg: u16, offset: u16, src: MemoryAddress) {
        let ma = MemoryAddress::RealSegmentOffset(seg, offset);
        for seen in &mut self.seen_destinations {
            if seen.address.value() == ma.value() {
                if DEBUG_TRACER {
                    println!("learn_destination src [{:04X}:{:04X}]", seg, offset);
                }
                seen.sources.push(src);
                return;
            }
        }
        if DEBUG_TRACER {
            println!("learn_destination dst [{:04X}:{:04X}]", seg, offset);
        }
        self.seen_destinations.push(SeenDestination{address: ma, visited: false, sources: vec!(src)});
    }

    fn get_sources_for_destination(&self, ma: MemoryAddress) -> Option<Vec<MemoryAddress>> {
        for dst in &self.seen_destinations {
            if dst.address.value() == ma.value() {
                if dst.sources.len() == 0 {
                    return None;
                }
                return Some(dst.sources.clone());
            }
        }
        None
    }

    fn has_any_unvisited_destinations(&self) -> bool {
        for dst in &self.seen_destinations {
            if !dst.visited {
                return true;
            }
        }
        false
    }

    fn get_unvisited_destination(&self) -> Option<MemoryAddress> {
        for dst in &self.seen_destinations {
            if !dst.visited {
                return Some(dst.address);
            }
        }
        None
    }

    fn mark_destination_visited(&mut self, ma: MemoryAddress) {
         for dst in &mut self.seen_destinations {
            if dst.address == ma {
                if DEBUG_TRACER {
                    println!("mark_destination_visited {:04X}:{:04X}", ma.segment(), ma.offset());
                }
                dst.visited = true;
                return;
            }
        }
    }

    fn has_visited_address(&self, ma: MemoryAddress) -> bool {
        for visited in &self.visited_addresses {
            if visited.value() == ma.value() {
                return true;
            }
        }
        false
    }

    /// traces along one execution path until we have to give up, marking it as visited when complete
    fn trace_unvisited_destination(&mut self, machine: &mut Machine) {
        // find a non-visited seen dest
        let ma = self.get_unvisited_destination();
        if let None = ma {
            println!("ERROR: no destinations to visit");
            return;
        }
        let mut ma = ma.unwrap();
        let start_ma = ma;

        // if destination has been visited, mark and return
        if self.has_visited_address(ma) {
            if DEBUG_TRACER {
                println!("We've already visited {:04X}:{:04X} == {:06X}, marking destination visited!", ma.segment(), ma.offset(), ma.value());
            }
            self.mark_destination_visited(start_ma);
            return;
        }

        if DEBUG_TRACER {
            println!("trace_destination starting at {:04X}:{:04X}", ma.segment(), ma.offset());
        }

        let mut decoder = Decoder::default();

        loop {
            let ii = decoder.get_instruction_info(&mut machine.hw.mmu, ma.segment(), ma.offset());
            if DEBUG_TRACER {
                println!("Found {}", ii);
            }

            if self.has_visited_address(ma) {
                if DEBUG_TRACER {
                    println!("already been here! breaking");
                }
                break;
            }

            // mark visited_address
            self.visited_addresses.push(ma);

            match ii.instruction.command {
                Op::Invalid(_, _) => println!("ERROR: invalid/unhandled op {}", ii.instruction),
                Op::RetImm16 => panic!("XXX unhandled {}", ii.instruction),
                Op::Retn | Op::Retf => break,
                Op::JmpNear | Op::JmpFar | Op::JmpShort => {
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_destination(ma.segment(), imm, ma),
                        Parameter::Reg16(_) => {}, // ignore "jmp bx"
                        Parameter::Ptr16(_, _) => {}, // ignore "jmp [0x4422]"
                        Parameter::Ptr16Imm(_, _) => {}, // ignore "jmp far 0xFFFF:0x0000"
                        Parameter::Ptr16AmodeS8(_, _, _) => {}, // ignore "jmp [di+0x10]
                        Parameter::Ptr16AmodeS16(_, _, _) => {}, // ignore "jmp [si+0x662C]"
                        _ => println!("ERROR1: unhandled dst type {:?}: {}", ii.instruction, ii.instruction),
                    }
                    // if unconditional branch, abort trace this path
                    break;
                }
                Op::CallNear | Op::CallFar | Op::Loop | Op::Loope | Op::Loopne |
                Op::Ja | Op::Jc | Op::Jcxz | Op::Jg | Op::Jl |
                Op::Jna | Op::Jnc | Op::Jng | Op::Jnl | Op::Jno | Op::Jns | Op::Jnz |
                Op::Jo | Op::Jpe | Op::Jpo | Op::Js | Op::Jz => {
                    // if conditional branch, record dst offset for later
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_destination(ma.segment(), imm, ma),
                        Parameter::Reg16(_) => {}, // ignore "call bp"
                        Parameter::Ptr16(_, _) => {}, // ignore "call [0x4422]"
                        Parameter::Ptr16AmodeS8(_, _, _) => {}, // ignore "call [di+0x10]
                        Parameter::Ptr16AmodeS16(_, _, _) => {}, // ignore "call [bx-0x67A0]"
                        _ => println!("ERROR2: unhandled dst type {:?}: {}", ii.instruction, ii.instruction),
                    }
                }
                _ => {},
            }
            ma.inc_n(ii.instruction.length as u16);

            if (ma.offset() - machine.rom_base.offset()) as isize >= machine.rom_length as isize {
                println!("XXX breaking because we reached end of file at offset {:04X}:{:04X} (indicates incorrect parsing)", ma.segment(), ma.offset());
                break;
            }
        }
        self.mark_destination_visited(start_ma);
    }
}
