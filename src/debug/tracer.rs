use std::cmp;
use std::fmt;

use machine::Machine;
use cpu::{Decoder, RepeatMode, InstructionInfo, R, Op, Parameter};
use memory::MemoryAddress;
use string::right_pad;
use hex::hex_bytes;

#[cfg(test)]
#[path = "./tracer_test.rs"]
mod tracer_test;

const DEBUG_TRACER: bool = false;

/// ProgramTracer holds the state of the program being analyzed
#[derive(Default)]
pub struct ProgramTracer {
    seen_destinations: Vec<SeenDestination>,

    /// flat addresses of start of each visited opcode
    visited_addresses: Vec<MemoryAddress>,

    /// finalized analysis result
    accounted_bytes: Vec<GuessedDataAddress>,
}

struct SeenDestination {
    /// segment:offset converted into real flat addresses
    address: MemoryAddress,

    sources: Vec<SeenSource>,

    visited: bool,
}

#[derive(Clone, Eq, PartialEq)]
struct SeenSource {
    address: MemoryAddress,
    kind: AddressUsageKind,
}

impl PartialOrd for SeenSource {
    fn partial_cmp(&self, other: &SeenSource) -> Option<cmp::Ordering> {
        Some(other.cmp(self))
    }
}

impl Ord for SeenSource {
    fn cmp(&self, other: &SeenSource) -> cmp::Ordering {
        other.address.value().cmp(&self.address.value())
    }
}


#[derive(Clone, Eq, PartialEq)]
enum GuessedDataType {
    InstrStart,
    InstrContinuation,
    UnknownByte(u8),
}

#[derive(Eq, PartialEq)]
struct GuessedDataAddress {
    kind: GuessedDataType,
    address: MemoryAddress,
}

#[derive(Clone, PartialEq, Eq)]
enum AddressUsageKind {
    Branch,
    Call,
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

impl ProgramTracer {
    pub fn default() -> Self {
        ProgramTracer {
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
        let mut decoder = Decoder::default();

        // walk each byte of the loaded rom and check w instr lengths
        // if any bytes are not known to occupy, allows for us to show them as data
        for ma in &self.visited_addresses {
            // translate address into physical offset
            let abs = (ma.value() - u32::from(machine.rom_base.offset())) as usize;

            let ii = decoder.get_instruction_info(&mut machine.hw.mmu, ma.segment(), ma.offset());

            let mut adr = *ma;
            self.accounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::InstrStart, address: adr});
            if  DEBUG_TRACER {
                println!("add start instr at {}", adr);
            }
            for _ in abs + 1..(abs + ii.instruction.length as usize) {
                adr.inc_u8();
                self.accounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::InstrContinuation, address: adr});
                if  DEBUG_TRACER {
                    println!("add continuation instr at {}", adr);
                }
            }
        }

        // find all unvisited offsets
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
                let val = machine.hw.mmu.read_u8(adr.segment(), adr.offset());
                unaccounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::UnknownByte(val), address: adr});
            }
        }

        for ub in unaccounted_bytes {
            self.accounted_bytes.push(ub);
        }

        self.accounted_bytes.sort();
    }

    /// implementation is in src/hardware.rs in_u8()
    fn in_u8_port_desc(&self, port: u16) -> String {
        match port {
            0x0060 => "keyboard or kb controller data output buffer".to_owned(),
            0x0061 => "keyboard controller port B control register".to_owned(),
            _ => {
                format!("XXX in_u8_port_desc unrecognized port {:04X}", port)
            },
        }
    }

    fn in_u16_port_desc(&self, port: u16) -> String {
        match port {
            _ => {
                format!("XXX in_u16_port_desc unrecognized port {:04X}", port)
            },
        }
    }

    fn decorate_instruction(&self, ii: &InstructionInfo) -> String {
        match ii.instruction.command {
            Op::In8 => {
                // E460              in al,0x60
                match ii.instruction.params.src {
                    Parameter::Imm8(port) => self.in_u8_port_desc(port as u16),
                    _ => "".to_owned(),
                }
            }
            Op::In16 => {
                // E560              in ax,0x60
                match ii.instruction.params.src {
                    Parameter::Imm8(port) => self.in_u16_port_desc(port as u16),
                    _ => "".to_owned(),
                }
            }
            Op::Stosb => {
                match ii.instruction.repeat {
                    RepeatMode::Rep => "store al at es:di for cx times".to_owned(),
                    RepeatMode::None => "store al at es:di".to_owned(),
                    _ => "".to_owned(),
                }
            }
            Op::Stosw => {
                match ii.instruction.repeat {
                    RepeatMode::Rep => "store ax at es:di for cx times".to_owned(),
                    RepeatMode::None => "store ax at es:di".to_owned(),
                    _ => "".to_owned(),
                }
            }
            _ => "".to_owned(),
        }
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

                    let mut tail = String::new();
                    let xref = self.render_xref(ab.address);
                    if xref != "" {
                        tail.push_str(&xref);
                    }

                    let decor = self.decorate_instruction(&ii);
                    if decor != "" {
                        tail.push_str(&format!("; {}", decor));
                    }

                    if tail != "" {
                        res.push_str(&format!("{}{}", right_pad(&format!("{}", ii), 68), tail));
                    } else {
                        let iis = format!("{}", ii);
                        res.push_str(&iis);
                    }
                    res.push('\n');

                    let mut next = ab.address.clone();
                    next.inc_n(ii.instruction.length as u16);

                    if self.is_call_dst(next) || ii.instruction.is_ret() || ii.instruction.is_unconditional_jmp() {
                        res.push('\n');
                    }
                }
                GuessedDataType::InstrContinuation => {},
                GuessedDataType::UnknownByte(val) => {
                    res.push_str(&format!("[{}] {:02X}               db       0x{:02X}\n", ab.address, val, val));
                }
            }
        }

        res
    }

    fn is_call_dst(&self, ma: MemoryAddress) -> bool {
        if let Some(sources) = self.get_sources_for_destination(ma) {
            for src in &sources {
                if src.kind == AddressUsageKind::Call {
                    return true;
                }
            }
        }
        false
    }

    /// show branch cross references
    fn render_xref(&self, ma: MemoryAddress) -> String {
        let mut s = String::new();
        if let Some(mut sources) = self.get_sources_for_destination(ma) {
            sources.sort();
            let mut source_offsets = Vec::new();
            for src in &sources {
                let label = match src.kind {
                    AddressUsageKind::Branch => "branch",
                    AddressUsageKind::Call => "call",
                };
                source_offsets.push(format!("{}@{}", label, src.address));
            }
            s = format!("; xref: {}", source_offsets.join(", "));
        }

        s
    }

    fn learn_destination(&mut self, seg: u16, offset: u16, src: MemoryAddress, kind: AddressUsageKind) {
        let ma = MemoryAddress::RealSegmentOffset(seg, offset);
        for seen in &mut self.seen_destinations {
            if seen.address.value() == ma.value() {
                if DEBUG_TRACER {
                    println!("learn_destination src [{:04X}:{:04X}]", seg, offset);
                }
                seen.sources.push(SeenSource{address: src, kind: kind});
                return;
            }
        }
        if DEBUG_TRACER {
            println!("learn_destination dst [{:04X}:{:04X}]", seg, offset);
        }
        self.seen_destinations.push(SeenDestination{address: ma, visited: false, sources: vec!(SeenSource{address: src, kind: kind})});
    }

    fn get_sources_for_destination(&self, ma: MemoryAddress) -> Option<Vec<SeenSource>> {
        for dst in &self.seen_destinations {
            if dst.address.value() == ma.value() {
                if dst.sources.is_empty() {
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
        let ma = self.get_unvisited_destination();
        if ma.is_none() {
            println!("ERROR: no destinations to visit");
            return;
        }
        let mut ma = ma.unwrap();
        let start_ma = ma;

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

            self.visited_addresses.push(ma);

            match ii.instruction.command {
                Op::Invalid(_, _) => println!("ERROR: invalid/unhandled op {}", ii.instruction),
                Op::RetImm16 => panic!("XXX unhandled {}", ii.instruction),
                Op::Retn | Op::Retf => break,
                Op::JmpNear | Op::JmpFar | Op::JmpShort => {
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_destination(ma.segment(), imm, ma, AddressUsageKind::Branch),
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
                Op::Loop | Op::Loope | Op::Loopne |
                Op::Ja | Op::Jc | Op::Jcxz | Op::Jg | Op::Jl |
                Op::Jna | Op::Jnc | Op::Jng | Op::Jnl | Op::Jno | Op::Jns | Op::Jnz |
                Op::Jo | Op::Jpe | Op::Jpo | Op::Js | Op::Jz => {
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_destination(ma.segment(), imm, ma, AddressUsageKind::Branch),
                        Parameter::Reg16(_) => {}, // ignore "call bp"
                        Parameter::Ptr16(_, _) => {}, // ignore "call [0x4422]"
                        Parameter::Ptr16AmodeS8(_, _, _) => {}, // ignore "call [di+0x10]
                        Parameter::Ptr16AmodeS16(_, _, _) => {}, // ignore "call [bx-0x67A0]"
                        _ => println!("ERROR2: unhandled dst type {:?}: {}", ii.instruction, ii.instruction),
                    }
                }
                Op::CallNear | Op::CallFar => {
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_destination(ma.segment(), imm, ma, AddressUsageKind::Call),
                        Parameter::Reg16(_) => {}, // ignore "call bp"
                        Parameter::Ptr16(_, _) => {}, // ignore "call [0x4422]"
                        Parameter::Ptr16AmodeS8(_, _, _) => {}, // ignore "call [di+0x10]
                        Parameter::Ptr16AmodeS16(_, _, _) => {}, // ignore "call [bx-0x67A0]"
                        _ => println!("ERROR3: unhandled dst type {:?}: {}", ii.instruction, ii.instruction),
                    }
                }
                _ => {},
            }
            ma.inc_n(u16::from(ii.instruction.length));

            if (ma.offset() - machine.rom_base.offset()) as isize >= machine.rom_length as isize {
                println!("XXX breaking because we reached end of file at offset {:04X}:{:04X} (indicates incorrect parsing or more likely missing symbolic execution eg meaning of 'int 0x20')", ma.segment(), ma.offset());
                break;
            }

        }
        self.mark_destination_visited(start_ma);
    }
}
