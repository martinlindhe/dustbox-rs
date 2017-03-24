pub struct CPU {
    pub pc: u16,
    memory: [u8; 0x10000],
    r16: [u16; 8], // XXX instead use a register struct, which has 8-bit parts adressable so we can set AL to a value and AX gets it too
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
            r16: [0, 0, 0, 0, 0, 0, 0, 0],
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
                let reg = (b&7) as usize;
                println!("XXX IMPL: mov {}, {:04X}", reg, val);

                // XXX r8 which is pointers into the r16 struct ....
                println!("mov {}, {:02X}", b & 7, val);
            },
            0xB8...0xBF => {
                // mov r16, u16
                let val = self.read_u16();
                let reg = (b&7) as usize;
                self.r16[reg] = val;
            },
            0xCD => {
                // XXX jump to offset 0x21 in interrupt table (look up how hw does this)
                println!("XXX IMPL: int {:02X}", self.read_u8());
            },
            _ => println!("UNHANDLED OP {:02X} AT {:04X}", b, self.pc-1),
        };
    }

    pub fn print_registers(&mut self) {
        print!("pc:{:04X}  ax:{:04X} bx:{:04X} cx:{:04X} dx:{:04X}", self.pc, self.r16[AX], self.r16[BX], self.r16[CX], self.r16[DX]);
        println!("  sp:{:04X} bp:{:04X} si:{:04X} di:{:04X}",  self.r16[SP], self.r16[BP], self.r16[SI], self.r16[DI]);    }

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
