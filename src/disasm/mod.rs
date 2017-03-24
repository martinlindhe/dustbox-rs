
pub struct Disassembly {
    pub pc: u16,
    rom: Vec<u8>,
    memory: [u8; 0x10000],
}

struct Instruction {
    offset: u16,
    length: u16,
    text: String,
}

impl Disassembly {
    pub fn new() -> Disassembly {
        Disassembly {
            pc: 0,
            memory: [0; 0x10000],
            rom: vec![],
        }
    }

    // loads data into a 64k area starting at offset, then disassembles all of it
    pub fn disassemble(&mut self, data: &Vec<u8>, offset: u16) -> String {
        self.load_rom(data, offset);

        // TODO LATER: could use rust iter features

        let mut count = 0;
        let mut res = vec![];
        loop {
            let op = self.disasm_instruction();
            res.push(format!("{:04X}: {}", op.offset, op.text));
            count += op.length as usize;
            if count >= data.len() {
                break;
            }
        }

        res.join("\n")
    }

    fn load_rom(&mut self, data: &Vec<u8>, offset: u16) {
        self.rom = data.clone();
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
            self.memory[i] = self.rom[rom_pos];
        }
    }

    fn disasm_instruction(&mut self) -> Instruction {
        let b = self.memory[self.pc as usize];
        let offset = self.pc;
        self.pc += 1;
        let s = match b {
            0x48...0x4F => format!("dec {}", r16(b & 7)),
            0x8E => {
                // XXX
                // op.Cmd = "mov"
                // op.parameterSregRm16(data)
                format!("MOV XXXX SregRm16")
            }
            0xB0...0xB7 => format!("mov {}, {:02X}", r8(b & 7), self.read_u8()),
            0xB8...0xBF => format!("mov {}, {:04X}", r16(b & 7), self.read_u16()),
            0xCD => format!("int {:02X}", self.read_u8()),
            _ => format!("UNHANDLED OP {:02X} AT {:04X}", b, offset),
        };

        Instruction {
            offset: offset,
            length: self.pc - offset,
            text: s,
        }
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

fn r8(reg: u8) -> &'static str {
    match reg {
        0 => "al",
        1 => "cl",
        2 => "dl",
        3 => "bl",
        4 => "ah",
        5 => "ch",
        6 => "dh",
        7 => "bh",
        _ => "?",
    }
}
fn r16(reg: u8) -> &'static str {
    match reg {
        0 => "ax",
        1 => "cx",
        2 => "dx",
        3 => "bx",
        4 => "sp",
        5 => "bp",
        6 => "si",
        7 => "di",
        _ => "?",
    }
}
