#![allow(dead_code)]

pub struct CPU {
    pub pc: u16,
    memory: [u8; 0x10000],
    r16: [Register16; 8],
}

#[derive(Debug, Copy, Clone)]
struct Register16 {
    hi: u8,
    lo: u8,
}

impl Register16 {
    fn u16(&self) -> u16 {
        (self.hi as u16) << 8 | self.lo as u16
    }
}

// r16
const AX: usize = 0;
const CX: usize = 1;
const DX: usize = 2;
const BX: usize = 3;
const SP: usize = 4;
const BP: usize = 5;
const SI: usize = 6;
const DI: usize = 7;


impl CPU {
    pub fn new() -> CPU {
        CPU {
            pc: 0,
            memory: [0; 0x10000],
            r16: [Register16 { hi: 0, lo: 0 }; 8],
        }
    }

    pub fn reset(&mut self) {
        self.pc = 0;
    }

    pub fn load_rom(&mut self, data: &Vec<u8>, offset: u16) {
        self.pc = offset;

        // copy up to 64k of rom
        let mut max = (offset as usize) + data.len();
        if max > 0x10000 {
            max = 0x10000;
        }
        let min = offset as usize;
        println!("loading rom to {:04X}..{:04X}", min, max);

        for i in min..max {
            let rom_pos = i - (offset as usize);
            self.memory[i] = data[rom_pos];
        }
    }

    pub fn execute_instruction(&mut self) {
        let b = self.memory[self.pc as usize];
        self.pc += 1;
        match b {
            //0x48...0x4F => format!("dec {}", r16(b & 7)),
            0xB0...0xB7 => {
                let val = self.read_u8();
                let reg = (b & 7) as usize;

                let lor = reg & 3;
                if reg & 4 == 0 {
                    self.r16[lor].lo = val;
                } else {
                    self.r16[lor].hi = val;
                }
            }
            0xB8...0xBF => {
                // mov r16, u16
                let reg = (b & 7) as usize;
                self.r16[reg].lo = self.read_u8();
                self.r16[reg].hi = self.read_u8();
            }
            0xCD => {
                // XXX jump to offset 0x21 in interrupt table (look up how hw does this)
                println!("XXX IMPL: int {:02X}", self.read_u8());
            }
            _ => println!("UNHANDLED OP {:02X} AT {:04X}", b, self.pc - 1),
        };
    }

    pub fn print_registers(&mut self) {
        print!("pc:{:04X}  ax:{:04X} bx:{:04X} cx:{:04X} dx:{:04X}",
               self.pc,
               self.r16[AX].u16(),
               self.r16[BX].u16(),
               self.r16[CX].u16(),
               self.r16[DX].u16());
        println!("  sp:{:04X} bp:{:04X} si:{:04X} di:{:04X}",
                 self.r16[SP].u16(),
                 self.r16[BP].u16(),
                 self.r16[SI].u16(),
                 self.r16[DI].u16());
    }

    fn read_u8(&mut self) -> u8 {
        let b = self.memory[self.pc as usize];
        self.pc += 1;
        b
    }

    fn read_u16(&mut self) -> u16 {
        let lo = self.read_u8();
        let hi = self.read_u8();
        (hi as u16) << 8 | lo as u16
    }
}
