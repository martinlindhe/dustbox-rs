use dustbox::machine::Machine;
use dustbox::cpu::{Decoder, R, Op, Parameter};
use dustbox::memory::MemoryAddress;

struct SeenDestination {
    /// segment:offset converted into real flat addresses
    address: MemoryAddress,

    visited: bool,
}

pub struct Tracer {
    seen_destinations: Vec<SeenDestination>,

    /// flat addresses of start of each visited opcode
    visited_addresses: Vec<u32>,
}

impl Tracer {
    pub fn new() -> Self {
        Tracer {
            seen_destinations: Vec::new(),
            visited_addresses: Vec::new(),
        }
    }

    pub fn trace_execution(&mut self, machine: &mut Machine) {
        // tell tracer to start at CS:IP
        let ma = MemoryAddress::RealSegmentOffset(machine.cpu.get_r16(R::CS), machine.cpu.regs.ip);
        self.seen_destinations.push(SeenDestination{address: ma, visited: false});

        loop {
            self.trace_unvisited_destination(machine);
            if !self.has_any_unvisited_destinations() {
                // println!("exhausted all destinations, breaking!");
                break;
            }
        }
    }

    fn learn_destination(&mut self, seg: u16, offset: u16) {
        let ma = MemoryAddress::RealSegmentOffset(seg, offset);
        println!("learn_destination [{:04X}:{:04X}]", seg, offset);
        for seen in &self.seen_destinations {
            if seen.address.value() == ma.value() {
                // println!("address was already learned! [{:04X}:{:04X}] == {:06X}", seg, offset, dst);
                return;
            }
        }
        self.seen_destinations.push(SeenDestination{address: ma, visited: false});
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
                // println!("XXX mark_destination_visited {:04X}:{:04X}", ma.segment(), ma.offset());
                dst.visited = true;
                return;
            }
        }
    }

    fn has_visited_address(&self, address: u32) -> bool {
        for visited in &self.visited_addresses {
            if *visited == address {
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
            println!("XXX no more destinations to visit");
            return;
        }
        let mut ma = ma.unwrap();
        let start_ma = ma;

        // if destination has been visited, mark and return
        if self.has_visited_address(ma.value()) {
            // println!("We've already visited {:04X}:{:04X} == {:06X}, marking destination visited!", ma.segment(), ma.offset(), ma.value());
            self.mark_destination_visited(start_ma);
            return;
        }

        println!("trace_destination starting at {:04X}:{:04X}", ma.segment(), ma.offset());

        let mut decoder = Decoder::default();

        loop {
            let ii = decoder.get_instruction_info(&mut machine.hw.mmu, ma.segment(), ma.offset());
            println!("{}", ii);

            // mark visited_address
            self.visited_addresses.push(ma.value());

            match ii.instruction.command {
                Op::Invalid(_, _) => {
                    panic!("we hit invalid/unhandled op!");
                }
                Op::JmpFar | Op::CallFar | Op::RetImm16 => {
                    panic!("XXX unhandled {:?}", ii.instruction);
                }
                Op::Retn | Op::Retf => {
                    break;
                }
                Op::JmpNear | Op::JmpShort => {
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_destination(ma.segment(), imm),
                        _ => panic!("unhandled dst type {:?}", ii.instruction.params.dst),
                    }
                    // if op is a unconditional jmp, then dont trace after this offset
                    break;
                }
                Op::CallNear | Op::Loop | Op::Loope | Op::Loopne |
                Op::Ja | Op::Jc | Op::Jcxz | Op::Jg | Op::Jl |
                Op::Jna | Op::Jnc | Op::Jng | Op::Jnl | Op::Jno | Op::Jns | Op::Jnz |
                Op::Jo | Op::Jpe | Op::Jpo | Op::Js | Op::Jz => {
                    // if op is a conditional jmp or call, record dst offset
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_destination(ma.segment(), imm),
                        _ => panic!("unhandled dst type {:?}", ii.instruction.params.dst),
                    }
                }
                _ => {},
            }
            ma.inc_n(ii.instruction.length as u16);
            if ma.value() - machine.cpu.rom_base >= machine.cpu.rom_length {
                println!("XXX breaking because we reached end of file");
                break;
            }
        }
        self.mark_destination_visited(start_ma);

        // separate traces by empty line
        println!("");
    }
}
