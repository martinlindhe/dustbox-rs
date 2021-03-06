use std::cmp;
use std::num::Wrapping;

use crate::machine::Machine;
use crate::cpu::{Decoder, RepeatMode, InstructionInfo, RegisterState, R, Op, Invalid, Parameter, Segment};
use crate::memory::MemoryAddress;
use crate::string::right_pad;

#[cfg(test)]
#[path = "./tracer_test.rs"]
mod tracer_test;

const DEBUG_TRACER: bool = false;

const DEBUG_LEARN_ADDRESS: bool = false;

/// ProgramTracer holds the state of the program being analyzed
#[derive(Default)]
pub struct ProgramTracer {
    seen_addresses: Vec<SeenAddress>,

    /// flat addresses of start of each visited opcode
    visited_addresses: Vec<MemoryAddress>,

    /// finalized analysis result
    accounted_bytes: Vec<GuessedDataAddress>,

    /// traced register state
    regs: RegisterState,
    dirty_regs: DirtyRegisters,

    /// annotations for an address
    annotations: Vec<TraceAnnotation>,

    /// traced $-strings in memory which can be decoded in final pass
    dollar_strings: Vec<MemoryAddress>,
}

#[derive(Default)]
struct DirtyRegisters {
    pub gpr: [bool; 8 + 6 + 1],
    pub sreg16: [bool; 6],
}

impl DirtyRegisters {
    /// marks all registers as dirty
    pub fn all_dirty(&mut self) {
        for i in 0..=8 + 6 {
            self.gpr[i] = true;
        }
        for i in 0..6 {
            self.sreg16[i] = true;
        }
    }

    pub fn is_dirty(&self, r: R) -> bool {
        if r.is_gpr() {
            self.gpr[r.index()]
        } else {
            self.sreg16[r.index()]
        }
    }

    pub fn clean_r(&mut self, r: R) {
        if r.is_gpr() {
            self.gpr[r.index()] = false;
        } else {
            self.sreg16[r.index()] = false;
        }
    }

    pub fn dirty_r(&mut self, r: R) {
        if r.is_gpr() {
            self.gpr[r.index()] = true;
        } else {
            self.sreg16[r.index()] = true;
        }
    }
}

struct TraceAnnotation {
    ma: MemoryAddress,
    note: String,
}

#[derive(Debug)]
struct SeenAddress {
    ma: MemoryAddress,
    sources: SeenSources,
    visited: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SeenSources {
    sources: Vec<SeenSource>,
}

impl SeenSources {
    pub fn default() -> Self {
        SeenSources {
            sources: Vec::new(),
        }
    }

    /// returns true if any source says its code
    pub fn has_code(&self) -> bool {
        for src in &self.sources {
            if !src.kind.is_code() {
                return false;
            }
        }
        true
    }

    pub fn from_source(source: SeenSource) -> Self {
        let mut res = Vec::new();
        res.push(source);
        SeenSources {
            sources: res,
        }
    }

    /// returns true if the sources are only of memory access kind
    pub fn only_memory_access(&self) -> bool {
        for src in &self.sources {
            if !src.kind.is_memory_kind() {
                return false;
            }
        }
        true
    }

    pub fn guess_data_type(&self) -> GuessedDataType {
        let mut word_access = false;
        for src in &self.sources {
            if src.kind == AddressUsageKind::MemoryWord {
                word_access = true;
            }
        }
        if word_access {
            GuessedDataType::MemoryWordUnset
        } else {
            GuessedDataType::MemoryByteUnset
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
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


#[derive(Clone, Debug, Eq, PartialEq)]
enum GuessedDataType {
    InstrStart,
    InstrContinuation,
    MemoryByteUnset,
    MemoryWordUnset,
    UnknownBytes(Vec<u8>),

    /// $-terminated ascii string
    DollarStringStart(Vec<u8>,String),
    DollarStringContinuation,
}

#[derive(Debug, Eq, PartialEq)]
struct GuessedDataAddress {
    kind: GuessedDataType,
    address: MemoryAddress,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AddressUsageKind {
    Branch,
    Call,
    Jump,
    MemoryByte,
    MemoryWord,
    DollarString,
}

impl AddressUsageKind {
    pub fn is_memory_kind(&self) -> bool {
        match *self {
            AddressUsageKind::MemoryByte | AddressUsageKind::MemoryWord | AddressUsageKind::DollarString => true,
            _ => false,
        }
    }

    /// returns true if it's known the address holds code
    pub fn is_code(&self) -> bool {
        match *self {
            AddressUsageKind::Branch | AddressUsageKind::Call | AddressUsageKind::Jump => true,
            _ => false,
        }
    }
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
            seen_addresses: Vec::new(),
            visited_addresses: Vec::new(),
            accounted_bytes: Vec::new(),
            regs: RegisterState::default(),
            dirty_regs: DirtyRegisters::default(),
            annotations: Vec::new(),
            dollar_strings: Vec::new(),
        }
    }

    /// traces all discovered paths of the program by static analysis
    pub fn trace_execution(&mut self, machine: &mut Machine) {
        // init known register values at program start
        self.dirty_regs.all_dirty();
        self.trace_r16(R::CS, machine.cpu.get_r16(R::CS));
        self.trace_r16(R::DS, machine.cpu.get_r16(R::DS));
        self.trace_r16(R::ES, machine.cpu.get_r16(R::ES));
        self.trace_r16(R::SS, machine.cpu.get_r16(R::SS));

        // tell tracer to start at CS:IP
        let ma = MemoryAddress::RealSegmentOffset(machine.cpu.get_r16(R::CS), machine.cpu.regs.ip);
        self.seen_addresses.push(SeenAddress{ma, visited: false, sources: SeenSources::default()});

        println!("; starting tracing disassembly at {}", ma);

        loop {
            self.trace_unvisited_address(machine);
            if !self.has_unvisited_code_addresses() {
                if DEBUG_TRACER {
                    eprintln!("exhausted all destinations, breaking!");
                }
                break;
            }
        }

        self.post_process_execution(machine);
    }

    /// Performs final post-processing of the program trace.
    /// Produces the self.accounted_bytes vec
    fn post_process_execution(&mut self, machine: &mut Machine) {
        let mut decoder = Decoder::default();

        // account dollar-string bytes
        for adr in &self.dollar_strings {
            let mut adr = *adr;

            // build string of bytes starting at offset until $
            let mut dollars = String::new();
            let adr_start = adr;
            let mut data = Vec::new();

            for x in 0..100 {
                let val = machine.mmu.read_u8_addr(adr);
                data.push(val);
                if x > 0 {
                    self.accounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::DollarStringContinuation, address: adr});
                }
                adr.inc_n(1);
                dollars.push(val as char);

                if val == b'$' {
                    break;
                }
            }
            self.accounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::DollarStringStart(data, dollars), address: adr_start});
        }

        // walk each byte of the loaded rom and check w instr lengths
        // if any bytes are not known to occupy, allows for us to show them as data
        for ma in &self.visited_addresses {
            // translate address into physical offset
            let abs = (ma.value() - u32::from(machine.rom_base.offset())) as usize;

            let ii = decoder.get_instruction_info(&mut machine.mmu, ma.segment(), ma.offset());

            let mut adr = *ma;
            self.accounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::InstrStart, address: adr});
            if  DEBUG_TRACER {
                // eprintln!("add start instr at {}", adr);
            }
            for _ in abs + 1..(abs + ii.instruction.length as usize) {
                adr.inc_u8();
                self.accounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::InstrContinuation, address: adr});
                if  DEBUG_TRACER {
                    // eprintln!("add continuation instr at {}", adr);
                }
            }
        }

        // find all unvisited offsets
        let mut unaccounted_bytes = vec![];
        let mut block = Vec::new();
        let mut block_start = MemoryAddress::Unset;
        let mut block_last = MemoryAddress::Unset;
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
                    eprintln!("address is unaccounted {}", adr);
                }

                // determine if last byte was in this range
                if let MemoryAddress::RealSegmentOffset(_seg, off) = block_last {
                    if off != adr.offset().wrapping_sub(1) && !block.is_empty() {
                        unaccounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::UnknownBytes(block.clone()), address: block_start});
                        block.clear();
                    }
                }
                if block.is_empty() {
                    block_start = adr;
                }
                block_last = adr;

                let val = machine.mmu.read_u8(adr.segment(), adr.offset());
                block.push(val);

                if block.len() >= 4 {
                    unaccounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::UnknownBytes(block.clone()), address: block_start});
                    block.clear();
                }
            }
        }

        if !block.is_empty() {
            unaccounted_bytes.push(GuessedDataAddress{kind: GuessedDataType::UnknownBytes(block), address: block_start});
        }

        for ub in unaccounted_bytes {
            self.accounted_bytes.push(ub);
        }

        // find all unvisited memory addresses
        for dst in &self.seen_addresses {
            if !dst.visited && !dst.sources.has_code() && !self.did_account_for(dst.ma) {
                let kind = dst.sources.guess_data_type();
                self.accounted_bytes.push(GuessedDataAddress{kind, address: dst.ma});
            }
        }

        self.accounted_bytes.sort();
    }

    /// returns true if ma exists in self.accounted_bytes
    fn did_account_for(&self, ma: MemoryAddress) -> bool {
        for ab in &self.accounted_bytes {
            if ab.address == ma {
                return true;
            }
        }
        false
    }

    /// returns a instruction annotation
    fn annotate_instruction(&self, ii: &InstructionInfo) -> String {
        match ii.instruction.command {
            Op::Lodsb => {
                match ii.instruction.repeat {
                    RepeatMode::None => "al = [ds:si]".to_owned(),
                    _ => "xxx Lodsb".to_owned(),
                }
            }
            Op::Lodsw => {
                match ii.instruction.repeat {
                    RepeatMode::None => "ax = [ds:si]".to_owned(),
                    _ => "xxx Lodsw".to_owned(),
                }
            }
            Op::Stosb => {
                match ii.instruction.repeat {
                    RepeatMode::Rep => "while cx-- > 0 { [es:di] = al }".to_owned(),
                    RepeatMode::None => "[es:di] = al".to_owned(),
                    _ => "xxx Stosb".to_owned(),
                }
            }
            Op::Stosw => {
                match ii.instruction.repeat {
                    RepeatMode::Rep => "while cx-- > 0 { [es:di] = ax }".to_owned(),
                    RepeatMode::None => "[es:di] = ax".to_owned(),
                    _ => "xxx Stosw".to_owned(),
                }
            }
            _ => {
                let v: Vec<&TraceAnnotation> = self.annotations.iter()
                    .filter(|a| a.ma == MemoryAddress::RealSegmentOffset(ii.segment as u16, ii.offset as u16))
                    .collect();

                let strs: Vec<String> = v.iter().map(|ta| ta.note.to_string()).collect();
                strs.join(" | ")
            }
        }
    }

    /// presents a traced disassembly listing
    pub fn present_trace(&mut self, machine: &mut Machine) -> String {

        // Displays decoded instructions at the known instruction offsets
        let mut decoder = Decoder::default();
        let mut res = String::new();

        for ab in &self.accounted_bytes {
            match &ab.kind {
                GuessedDataType::InstrStart => {
                    let ii = decoder.get_instruction_info(&mut machine.mmu, ab.address.segment(), ab.address.offset());

                    let mut tail = self.render_xref(ab.address);

                    let decor = self.annotate_instruction(&ii);
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

                    let mut next = ab.address;
                    next.inc_n(u16::from(ii.instruction.length));

                    if self.is_call_dst(next) || ii.instruction.is_ret() || ii.instruction.is_unconditional_jmp() || ii.instruction.is_loop() {
                        res.push('\n');
                    }
                }
                GuessedDataType::InstrContinuation => {},
                GuessedDataType::MemoryByteUnset => {
                    let xref = self.render_xref(ab.address);
                    res.push_str(&format!("[{}] ??               db       ??                            {}\n", ab.address, xref));
                }
                GuessedDataType::MemoryWordUnset => {
                    let xref = self.render_xref(ab.address);
                    res.push_str(&format!("[{}] ????             dw       ????                          {}\n", ab.address, xref));
                }
                GuessedDataType::UnknownBytes(v) => {
                    let tail = self.render_xref(ab.address);
                    let hex: Vec<String> = v.iter().map(|b| format!("{:02X}", b)).collect();
                    let pretty: Vec<String> = v.iter().map(|b| format!("0x{:02X}", b)).collect();
                    let mut s = format!("[{}] {:11}      db       {}", ab.address, hex.join(""), pretty.join(", "));
                    if tail != "" {
                        s.push_str(&format!("                          {}", tail));
                    }
                    s.push_str("\n");
                    res.push_str(&s);
                }
                GuessedDataType::DollarStringStart(v, s) => {
                    let xref = self.render_xref(ab.address);
                    let hex: Vec<String> = v.iter().map(|b| format!("{:02X}", b)).collect();
                    res.push_str(&format!("[{}] {:11}      db       '{}'                         {}\n", ab.address, hex.join(""), s, xref));
                }
                GuessedDataType::DollarStringContinuation => {},
            }
        }

        res
    }

    /// returns true if anyone called to given MemoryAddress
    fn is_call_dst(&self, ma: MemoryAddress) -> bool {
        if let Some(sources) = self.get_sources_for_address(ma) {
            if sources.has_code() {
                for src in &sources.sources {
                    if src.kind == AddressUsageKind::Call {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// show branch cross references
    fn render_xref(&self, ma: MemoryAddress) -> String {
        let mut s = String::new();
        if let Some(mut sources) = self.get_sources_for_address(ma) {
            sources.sources.sort();
            let mut source_offsets = Vec::new();
            for src in &sources.sources {
                let label = match src.kind {
                    AddressUsageKind::Branch => "branch",
                    AddressUsageKind::Jump => "jump",
                    AddressUsageKind::Call => "call",
                    AddressUsageKind::MemoryByte => "byte",
                    AddressUsageKind::MemoryWord => "word",
                    AddressUsageKind::DollarString => "str$",
                };
                source_offsets.push(format!("{}@{}", label, src.address));
            }
            s = format!("; xref: {}", source_offsets.join(", "));
        }
        s
    }

    // learns of a new address to probe later (creates a xref)
    fn learn_address(&mut self, seg: u16, offset: u16, src: MemoryAddress, kind: AddressUsageKind) {
        let ma = MemoryAddress::RealSegmentOffset(seg, offset);
        for seen in &mut self.seen_addresses {
            if seen.ma.value() == ma.value() {
                if DEBUG_LEARN_ADDRESS {
                    eprintln!("learn_address append {:?} [{:04X}:{:04X}]", kind, seg, offset);
                }
                seen.sources.sources.push(SeenSource{address: src, kind});
                return;
            }
        }
        if DEBUG_LEARN_ADDRESS {
            eprintln!("learn_address new {:?} [{:04X}:{:04X}]", kind, seg, offset);
        }
        self.seen_addresses.push(SeenAddress{ma, visited: false, sources: SeenSources::from_source(SeenSource{address: src, kind})});
    }

    fn get_sources_for_address(&self, ma: MemoryAddress) -> Option<SeenSources> {
        for dst in &self.seen_addresses {
            if dst.ma.value() == ma.value() {
                if dst.sources.sources.is_empty() {
                    return None;
                }
                return Some(dst.sources.clone());
            }
        }
        None
    }

    // returns true if we know about unvisited code addresses
    fn has_unvisited_code_addresses(&self) -> bool {
        for dst in &self.seen_addresses {
            if dst.sources.has_code() && !dst.visited {
                return true;
            }
        }
        false
    }

    // returns a unvisted address pointing to code
    fn get_unvisited_code_address(&self) -> Option<MemoryAddress> {
        for dst in &self.seen_addresses {
            if dst.sources.has_code() && !dst.visited {
                return Some(dst.ma);
            }
        }
        None
    }

    /// marks given seen address as visited by the prober
    fn mark_address_visited(&mut self, ma: MemoryAddress) {
         for dst in &mut self.seen_addresses {
            if dst.ma == ma {
                if DEBUG_TRACER {
                    eprintln!("mark_destination_visited {:04X}:{:04X}", ma.segment(), ma.offset());
                }
                dst.visited = true;
                return;
            }
        }
        panic!("never found address to mark as visited! {}", ma);
    }

    fn has_visited_address(&self, ma: MemoryAddress) -> bool {
        for visited in &self.visited_addresses {
            if visited.value() == ma.value() {
                return true;
            }
        }
        false
    }

    /// debug fn - print register value or dirty state
    fn print_register_state(&self) {
        println!("ax:{} bx:{} cx:{} dx:{}", self.traced_r_desc(R::AX), self.traced_r_desc(R::BX), self.traced_r_desc(R::CX), self.traced_r_desc(R::DX));
        println!("sp:{} bp:{} si:{} di:{}", self.traced_r_desc(R::SP), self.traced_r_desc(R::BP), self.traced_r_desc(R::SI), self.traced_r_desc(R::DI));
        println!("cs:{} ss:{} ds:{} es:{}", self.traced_r_desc(R::CS), self.traced_r_desc(R::SS), self.traced_r_desc(R::DS), self.traced_r_desc(R::ES));
        println!("fs:{} gs:{}", self.traced_r_desc(R::FS), self.traced_r_desc(R::GS));
    }

    fn traced_r_desc(&self, r: R) -> String {
        // XXX impl
        if let Some(v) = self.clean_r(r) {
            format!("{:04X}", v)
        } else {
            "dirty".to_string()
        }
    }

    /// sets and trace reg
    fn trace_r8(&mut self, r: R, val: u8) {
        // TODO mark 16-bit gpr dirty
        self.dirty_regs.clean_r(r);
        self.regs.set_r8(r, val);
    }

    /// sets and trace reg
    fn trace_r16(&mut self, r: R, val: u16) {
        self.dirty_regs.clean_r(r);
        self.regs.set_r16(r, val);
    }

    /// returns value of clean register or None
    fn clean_r(&self, r: R) -> Option<u16> {
        // XXX TODO if 8bit, see that parent 16-bit is clean
        if r.is_gpr() {
            if r.is_8bit() {
                if !self.dirty_regs.gpr[r.index()] {
                    return Some(u16::from(self.regs.get_r8(r)));
                }
            } else if !self.dirty_regs.gpr[r.index()] {
                return Some(self.regs.get_r16(r));
            }
        } else if !self.dirty_regs.sreg16[r.index()] {
            return Some(self.regs.get_r16(r));
        }
        None
    }

    /// traces along one execution path until we have to give up, marking it as visited when complete
    fn trace_unvisited_address(&mut self, machine: &mut Machine) {
        let ma = self.get_unvisited_code_address();
        if ma.is_none() {
            eprintln!("ERROR: no destinations to visit");
            return;
        }
        let mut ma = ma.unwrap();
        let start_ma = ma;

        if self.has_visited_address(ma) {
            if DEBUG_TRACER {
                eprintln!("We've already visited {:04X}:{:04X} == {:06X}, marking destination visited!", ma.segment(), ma.offset(), ma.value());
            }
            self.mark_address_visited(start_ma);
            return;
        }

        if DEBUG_TRACER {
            eprintln!("trace_destination starting at {:04X}:{:04X}", ma.segment(), ma.offset());
        }

        let mut decoder = Decoder::default();

        loop {
            let ii = decoder.get_instruction_info(&mut machine.mmu, ma.segment(), ma.offset());
            if DEBUG_TRACER {
                eprintln!("Found {}", ii);
            }

            if self.has_visited_address(ma) {
                if DEBUG_TRACER {
                    eprintln!("already been here! breaking");
                }
                break;
            }

            self.visited_addresses.push(ma);

            match ii.instruction.command {
                Op::Invalid(_, ref kind) => {
                    match kind {
                        Invalid::Op => eprintln!("ERROR: invalid/unhandled op {}", ii.instruction),
                        Invalid::FPUOp => eprintln!("ERROR: invalid/unhandled FPU op {}", ii.instruction),
                        Invalid::Reg(_) => eprintln!("ERROR: invalid/unhandled reg op {}", ii.instruction)
                    }
                },
                Op::RetImm16 => panic!("FIXME handle {}", ii.instruction),
                Op::Retn | Op::Retf => break,
                Op::JmpNear | Op::JmpFar | Op::JmpShort => {
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_address(ma.segment(), imm, ma, AddressUsageKind::Jump),
                        Parameter::Reg16(_) => {}, // ignore "jmp bx"
                        Parameter::Ptr16(_, _) => {}, // ignore "jmp [0x4422]"
                        Parameter::Ptr16Imm(_, _) => {}, // ignore "jmp far 0xFFFF:0x0000"
                        Parameter::Ptr16Amode(_, _) => {}, // ignore "2EFF27            jmp [cs:bx]"
                        Parameter::Ptr16AmodeS8(_, _, _) => {}, // ignore "jmp [di+0x10]
                        Parameter::Ptr16AmodeS16(_, _, _) => {}, // ignore "jmp [si+0x662C]"
                        _ => eprintln!("ERROR1: unhandled dst type {:?}: {}", ii.instruction, ii.instruction),
                    }
                    // if unconditional branch, abort trace this path
                    break;
                }
                Op::Loop | Op::Loope | Op::Loopne |
                Op::Ja | Op::Jc | Op::Jcxz | Op::Jg | Op::Jl |
                Op::Jna | Op::Jnc | Op::Jng | Op::Jnl | Op::Jno | Op::Jns | Op::Jnz |
                Op::Jo | Op::Jpe | Op::Jpo | Op::Js | Op::Jz => match ii.instruction.params.dst {
                    Parameter::Imm16(imm) => self.learn_address(ma.segment(), imm, ma, AddressUsageKind::Branch),
                    Parameter::Reg16(_) => {}, // ignore "call bp"
                    Parameter::Ptr16(_, _) => {}, // ignore "call [0x4422]"
                    Parameter::Ptr16AmodeS8(_, _, _) => {}, // ignore "call [di+0x10]
                    Parameter::Ptr16AmodeS16(_, _, _) => {}, // ignore "call [bx-0x67A0]"
                    _ => eprintln!("ERROR2: unhandled dst type {:?}: {}", ii.instruction, ii.instruction),
                }
                Op::CallNear | Op::CallFar => match ii.instruction.params.dst {
                    Parameter::Imm16(imm) => self.learn_address(ma.segment(), imm, ma, AddressUsageKind::Call),
                    Parameter::Reg16(_) => {}, // ignore "call bp"
                    Parameter::Ptr16(_, _) => {}, // ignore "call [0x4422]"
                    Parameter::Ptr16Imm(_, _) => {} // ignore "call 0x4422:0x3050"
                    Parameter::Ptr16Amode(_, _) => {}, // ignore "FF1F              call far [bx]"
                    Parameter::Ptr16AmodeS8(_, _, _) => {}, // ignore "call [di+0x10]
                    Parameter::Ptr16AmodeS16(_, _, _) => {}, // ignore "call [bx-0x67A0]"
                    _ => eprintln!("ERROR3: unhandled dst type {:?}: {}", ii.instruction, ii.instruction),
                }
                Op::Int => if let Parameter::Imm8(v) = ii.instruction.params.dst {
                    let ah = self.regs.get_r8(R::AH);

                    self.annotations.push(TraceAnnotation{ma, note: self.int_desc(v)});
                    self.annotations.push(TraceAnnotation{ma, note: "dirty all regs".to_owned()});

                    if v == 0x20 {
                        // int 0x20: exit to dos
                        break
                    }
                    if v == 0x21 && ah == 0x4C {
                        // int 0x21, 0x4C: exit to dos
                        break
                    }
                    if v == 0x21 && ah == 0x09 {
                        // track address for $-string in DS:DX
                        self.print_register_state();
                        if let Some(ds) = self.clean_r(R::DS) {
                            if let Some(dx) = self.clean_r(R::DX) {
                                self.learn_address(ds, dx, ma, AddressUsageKind::DollarString);
                                self.dollar_strings.push(MemoryAddress::RealSegmentOffset(ds, dx));
                            }
                        }
                    }

                    self.dirty_regs.all_dirty();
                }
                Op::Out8 | Op::Out16 => {
                    // TODO skip if register is dirty
                    let dst = match ii.instruction.params.dst {
                        Parameter::Imm8(v) => Some(u16::from(v)),
                        Parameter::Reg16(r) => Some(self.regs.get_r16(r)),
                        _ => None
                    };
                    if let Some(dst) = dst {
                        match ii.instruction.params.src {
                            Parameter::Reg8(r) => self.annotations.push(TraceAnnotation{
                                ma, note: format!("{} (0x{:04X}) = {:02X}", self.out_desc(dst as u16), dst, self.regs.get_r8(r))}),
                            Parameter::Reg16(r) => self.annotations.push(TraceAnnotation{
                                ma, note: format!("{} (0x{:04X}) = {:04X}", self.out_desc(dst as u16), dst, self.regs.get_r16(r))}),
                            _ => {}
                        }
                    }
                }
                Op::In8 | Op::In16 => {
                    // TODO skip if register is dirty
                    let src = match ii.instruction.params.src {
                        Parameter::Imm8(v) => Some(u16::from(v)),
                        Parameter::Reg16(r) => Some(self.regs.get_r16(r)),
                        _ => None
                    };
                    // TODO mark dst register dirty
                    if let Some(src) = src {
                        match ii.instruction.params.dst {
                            Parameter::Reg8(_) => self.annotations.push(TraceAnnotation{
                                ma, note: format!("{} (0x{:04X})", self.in_desc(src as u16), src)}),
                            Parameter::Reg16(_) => self.annotations.push(TraceAnnotation{
                                ma, note: format!("{} (0x{:04X})", self.in_desc(src as u16), src)}),
                            _ => {}
                        }
                    }
                }
                Op::Xor8 => if let Parameter::Reg8(dr) = ii.instruction.params.dst {
                    match ii.instruction.params.src {
                        Parameter::Reg8(sr) => if dr == sr {
                            self.trace_r8(dr, 0);
                            self.dirty_regs.clean_r(dr);
                            self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:02X}", dr, 0)});
                        }
                        _ => {}
                    }
                }
                Op::Xor16 => if let Parameter::Reg16(dr) = ii.instruction.params.dst {
                    match ii.instruction.params.src {
                        Parameter::Reg16(sr) => if dr == sr {
                            self.trace_r16(dr, 0);
                            self.dirty_regs.clean_r(dr);
                            self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:04X}", dr, 0)});
                        }
                        _ => {}
                    }
                }
                Op::Dec8 => if let Parameter::Reg8(dr) = ii.instruction.params.dst {
                    let v =(Wrapping(self.regs.get_r8(dr)) - Wrapping(1)).0;
                    self.trace_r8(dr, v);
                    self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:02X}", dr, v)});
                }
                Op::Dec16 => if let Parameter::Reg16(dr) = ii.instruction.params.dst {
                    let v = (Wrapping(self.regs.get_r16(dr)) - Wrapping(1)).0;
                    self.trace_r16(dr, v);
                    self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:04X}", dr, v)});
                }
                Op::Inc8 => if let Parameter::Reg8(dr) = ii.instruction.params.dst {
                    let v =(Wrapping(self.regs.get_r8(dr)) + Wrapping(1)).0;
                    self.trace_r8(dr, v);
                    self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:02X}", dr, v)});
                }
                Op::Inc16 => if let Parameter::Reg16(dr) = ii.instruction.params.dst {
                    let v = (Wrapping(self.regs.get_r16(dr)) + Wrapping(1)).0;
                    self.trace_r16(dr, v);
                    self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:04X}", dr, v)});
                }
                Op::Add8 => if let Parameter::Reg8(dr) = ii.instruction.params.dst {
                    // TODO skip if register is dirty
                    let v = match ii.instruction.params.src {
                        Parameter::Imm8(i) => Some(u16::from(i)),
                        Parameter::Reg8(sr) => self.clean_r(sr),
                        _ => None
                    };
                    if let Some(v) = v {
                        let v = (Wrapping(self.regs.get_r8(dr)) + Wrapping(v as u8)).0;
                        self.trace_r8(dr, v);
                        self.dirty_regs.clean_r(dr);
                        self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:02X}", dr, v)});
                    }
                }
                Op::Add16 => if let Parameter::Reg16(dr) = ii.instruction.params.dst {
                    // TODO skip if register is dirty
                    let v = match ii.instruction.params.src {
                        Parameter::ImmS8(i) => Some(i as u16), // XXX should be treated as signed
                        Parameter::Imm16(i) => Some(i),
                        Parameter::Reg16(sr) => self.clean_r(sr),
                        _ => None
                    };
                    if let Some(v) = v {
                        let v = (Wrapping(self.regs.get_r16(dr)) + Wrapping(v)).0;
                        self.trace_r16(dr, v);
                        self.dirty_regs.clean_r(dr);
                        self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:04X}", dr, v)});
                    }
                }
                Op::Sub8 => if let Parameter::Reg8(dr) = ii.instruction.params.dst {
                    // TODO skip if register is dirty
                    let v = match ii.instruction.params.src {
                        Parameter::Imm8(v) => Some(v),
                        Parameter::Reg8(sr) => Some(self.regs.get_r8(sr)),
                        _ => None
                    };
                    if let Some(v) = v {
                        let v = (Wrapping(self.regs.get_r8(dr)) - Wrapping(v)).0;
                        self.trace_r8(dr, v);
                        self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:02X}", dr, v)});
                    }
                }
                Op::Sub16 => if let Parameter::Reg16(dr) = ii.instruction.params.dst {
                    // TODO skip if register is dirty
                    let v = match ii.instruction.params.src {
                        Parameter::ImmS8(v) => Some(v as u16), // XXX should be treated as signed
                        Parameter::Imm16(v) => Some(v),
                        Parameter::Reg16(sr) => Some(self.regs.get_r16(sr)),
                        _ => None
                    };
                    if let Some(v) = v {
                        let v = (Wrapping(self.regs.get_r16(dr)) - Wrapping(v)).0;
                        self.trace_r16(dr, v);
                        self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:04X}", dr, v)});
                    }
                }
                Op::Mov8 | Op::Mov16 => {
                    match ii.instruction.params.dst {
                        Parameter::Reg8(r) => {
                            let v = match ii.instruction.params.src {
                                Parameter::Reg8(sr) => Some(self.regs.get_r8(sr)),
                                Parameter::Imm8(i) => Some(i),
                                _ => None
                            };
                            if let Some(v) = v {
                                self.trace_r8(r, v);
                                self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:02X}", r, v)});
                            }
                        }
                        Parameter::Reg16(r) | Parameter::SReg16(r) => {
                            let v = match ii.instruction.params.src {
                                Parameter::Reg16(sr) => self.clean_r(sr),
                                Parameter::Imm16(i) => Some(i),
                                _ => None
                            };
                            if let Some(v) = v {
                                self.trace_r16(r, v);
                                self.dirty_regs.clean_r(r);
                                self.annotations.push(TraceAnnotation{ma, note: format!("{} = 0x{:04X}", r, v)});
                            } else if let Parameter::Reg16(sr) = ii.instruction.params.src {
                                if self.dirty_regs.is_dirty(sr) {
                                    self.dirty_regs.dirty_r(r);
                                    self.annotations.push(TraceAnnotation{ma, note: format!("{} is dirty", r)});
                                }
                            }
                        }
                        Parameter::Ptr8(seg, offset) => {
                            // mov   [cs:0x0202], al
                            if seg == Segment::CS {
                                self.learn_address(machine.cpu.regs.get_r16(R::CS), offset, ma, AddressUsageKind::MemoryByte);
                            }
                        },
                        Parameter::Ptr16(seg, offset) => {
                            // mov   [cs:0x0202], ax
                            if seg == Segment::CS {
                                self.learn_address(machine.cpu.regs.get_r16(R::CS), offset, ma, AddressUsageKind::MemoryWord);
                            }
                        },
                        _ => {}
                    }

                    match ii.instruction.params.src {
                        Parameter::Ptr8(seg, offset) => {
                            // mov   al, [cs:0x0202]
                            if seg == Segment::CS {
                                self.learn_address(machine.cpu.regs.get_r16(R::CS), offset, ma, AddressUsageKind::MemoryByte);
                            }
                        },
                        Parameter::Ptr16(seg, offset) => {
                            // mov   ax, [cs:0x0202]
                            if seg == Segment::CS {
                                self.learn_address(machine.cpu.regs.get_r16(R::CS), offset, ma, AddressUsageKind::MemoryWord);
                            }
                        },
                        _ => {}
                    }
                }
                Op::Xchg8 | Op::Xchg16 |
                Op::And8 | Op::And16 |
                Op::Adc8 | Op::Adc16 |
                Op::Or8 | Op::Or16 |
                Op::Pop16 | Op::Pop32 |
                Op::Mul8 | Op::Mul16 |
                Op::Div8 | Op::Div16 |
                Op::Imul8 | Op::Imul16 |
                Op::Idiv8 | Op::Idiv16 |
                Op::Shl8 | Op::Shl16 | Op::Shld |
                Op::Shr8 | Op::Shr16 | Op::Shrd => {
                    // NOTE: several of these instructions could be simulated,
                    // but for now just mark dst registers as dirty.
                    match ii.instruction.params.dst {
                        Parameter::Reg8(r) | Parameter::Reg16(r) | Parameter::SReg16(r) => {
                            self.dirty_regs.dirty_r(r);
                            self.annotations.push(TraceAnnotation{ma, note: format!("{} is dirty", r)});
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            ma.inc_n(u16::from(ii.instruction.length));

            if (ma.offset() as isize - machine.rom_base.offset() as isize) >= machine.rom_length as isize {
                eprintln!("ERROR: breaking because we reached end of file at {} (indicates incorrect parsing)", ma);
                break;
            }
        }
        self.mark_address_visited(start_ma);
    }

    fn video_mode_desc(&self, mode: u8) -> &str {
        match mode {
            0x03 => "80x25x16 text",
            0x11 => "640x480x2",
            0x13 => "320x200x256",
            _ => "unrecognized"
        }
    }

    /// describe out port (write)
    fn out_desc(&self, port: u16) -> &str {
        match port {
            0x0040 => "pit: counter 0, counter divisor",
            0x0041 => "pit: counter 1, RAM refresh counter",
            0x0042 => "pit: counter 2, cassette & speaker",
            0x0060 => "keyboard: controller data port",
            0x0061 => "keyboard: controller port B",
            0x0201 => "joystick: fire four one-shots",
            0x03C4 => "vga: sequencer index register", // ega: TS index register
            0x03C6 => "vga: PEL mask register",
            0x03C7 => "vga: PEL address read mode",
            0x03C8 => "vga: PEL address write mode",
            0x03C9 => "vga: PEL data register",
            0x03D4 => "ega/vga: CRT (6845) index register",
            0x03DA => "ega/vga: feature control register",
            _ => "unrecognized",
        }
    }

    /// describe in port (read)
    fn in_desc(&self, port: u16) -> &str {
        match port {
            0x0040 => "pit: counter 0, counter divisor",
            0x0041 => "pit: counter 1, RAM refresh counter",
            0x0042 => "pit: counter 2, cassette & speaker",
            0x0060 => "keyboard: input buffer",
            0x0061 => "keyboard: controller port B control register",
            0x0201 => "joystick: read position and status",
            0x03C4 => "vga: sequencer index register",
            0x03C6 => "vga: PEL mask register",
            0x03C7 => "vga: PEL address read mode / vga: DAC state register",
            0x03C8 => "vga: PEL address write mode",
            0x03C9 => "vga: PEL data register",
            0x03DA => "ega/vga: input status 1 register", // cga: status register
            _ => "unrecognized",
        }
    }

    fn int_desc(&self, int: u8) -> String {
        let al = self.regs.get_r8(R::AL);
        let ah = self.regs.get_r8(R::AH);
        match int {
            0x10 => { // video
                match ah {
                    0x00 => format!("video: set {} mode (0x{:02X})", self.video_mode_desc(al), al),
                    0x02 => String::from("video: set cursor position"),
                    0x06 => String::from("video: scroll up"),
                    0x07 => String::from("video: scroll down"),
                    0x10 => match al {
                        0x12 => String::from("video: VIDEO - SET BLOCK OF DAC REGISTERS (VGA/MCGA)"),
                        _ => format!("video: unrecognized AH = 10, AL = {:02X}", al)
                    }
                    0x13 => String::from("video: write string (row=DH, col=DL)"),
                    _ => format!("video: unrecognized AH = {:02X}", ah)
                }
            }
            0x16 => { // keyboard
                match ah {
                    0x00 => String::from("keyboard: read scancode (blocking)"),
                    0x01 => String::from("keyboard: read scancode (non-blocking)"),
                    _ => format!("keyboard: unrecognized AH = {:02X}", ah)
                }
            }
            0x1A => { // pit timer
                match ah {
                    0x00 => String::from("pit: get system time"),
                    _ => format!("pit: unrecognized AH = {:02X}", ah)
                }
            }
            0x20 => {
                String::from("dos: terminate program with return code 0")
            }
            0x21 => { // DOS
                match ah {
                    0x02 => String::from("dos: write character in DL to stdout"),
                    0x06 => String::from("dos: write character in DL to DIRECT CONSOLE OUTPUT"),
                    0x09 => String::from("dos: write $-terminated string at DS:DX to stdout"),
                    0x4C => String::from("dos: terminate program with return code in AL"),
                    _ => format!("dos: unrecognized AH = {:02X}", ah)
                }
            }
            0x33 => { // mouse
                let al = self.regs.get_r8(R::AL);
                match al {
                     0x03 => String::from("mouse: get position and button status"),
                     _ => format!("mouse: unrecognized AL = {:02X}", al)
                }
            }
            _ => {
                format!("XXX int_desc unrecognized {:02X}", int)
            },
        }
    }
}
