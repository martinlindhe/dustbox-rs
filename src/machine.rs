use bincode::deserialize;

use gpu::GPU;
use cpu::{CPU, Op, InvalidOp, R, RegisterSnapshot, Segment, OperandSize};
use memory::MMU;
use hardware::Hardware;
use ndisasm::ndisasm_bytes;

#[derive(Deserialize, Debug)]
struct ExeHeader {
    signature: u16,             // 0x5A4D == "MZ"
    bytes_in_last_block: u16,   // padding info for exact data size
    blocks_in_file: u16,        // data size in 512-byte blocks
    num_relocs: u16,            // number of relocation items
    header_paragraphs: u16,     // header size in 16-byte paragraphs
    min_extra_paragraphs: u16,
    max_extra_paragraphs: u16,
    ss: u16,
    sp: u16,
    checksum: u16,
    ip: u16,
    cs: u16,
    reloc_table_offset: u16,
    overlay_number: u16,
}

#[derive(Deserialize, Debug)]
struct ExeReloc {
    offset: u16,
    segment: u16,
}

struct Exe {
    header: ExeHeader,
    relocs: Vec<ExeReloc>,
}

pub struct Machine {
    pub hw: Hardware,
    pub cpu: CPU,
}

impl Machine {
    pub fn default() -> Self {
        Machine {
            cpu: CPU::default(),
            hw: Hardware::default(),
        }
    }

   // reset the CPU and memory
    pub fn hard_reset(&mut self) {
        self.cpu = CPU::default();
    }

    pub fn load_executable(&mut self, data: &[u8]) {
        if data[0] == b'M' && data[1] == b'Z' {
            self.load_exe(data);
        } else {
            self.load_com(data);
        }
    }

    fn load_exe(&mut self, data: &[u8]) {
        let hdr: ExeHeader = deserialize(data).unwrap();
        println!("load_exe header: {:?}", hdr);

        let reloc_from = hdr.reloc_table_offset as usize;
        let reloc_to = hdr.reloc_table_offset as usize + (hdr.num_relocs as usize * 4);
        println!("load_exe read relocs from {:04X}-{:04X}", reloc_from, reloc_to);

        // let relocs: Vec<ExeReloc> = deserialize(&data[reloc_from..reloc_to]).unwrap();  // XXX crashes
        let relocs: ExeReloc = deserialize(&data[reloc_from..reloc_to]).unwrap(); // XXX only reads first reloc
        println!("XXX relocs: {:?}", relocs);

        let code_offset = hdr.header_paragraphs as usize * 16;
        let mut code_end = hdr.blocks_in_file as usize * 512;
        if hdr.bytes_in_last_block > 0 {
            code_end -= 512 - hdr.bytes_in_last_block as usize;
        }
        println!("load exe code from {:04X}:{:04X}", code_offset, code_end);

        self.load_com(&data[code_offset..code_end]);
        self.cpu.set_r16(&R::SP, hdr.sp); // confirmed
        self.cpu.set_r16(&R::SS, hdr.ss); // XXX dosbox = 0923
        
        // at program start in dosbox-x:
        // BP = 091C (dustbox ok)
        // SP = 1000 (from hdr, dustbox ok)
        // CS = 0920
        // DS = 0910
        // ES = 0910
        // SS = 0923
    }

    // load .com program into CS:0100 and set IP to program start
    fn load_com(&mut self, data: &[u8]) {
        // CS,DS,ES,SS = PSP segment
        let psp_segment = 0x085F; // is what dosbox used
        self.cpu.set_r16(&R::CS, psp_segment);
        self.cpu.set_r16(&R::DS, psp_segment);
        self.cpu.set_r16(&R::ES, psp_segment);
        self.cpu.set_r16(&R::SS, psp_segment);

        // offset of last word available in first 64k segment
        self.cpu.set_r16(&R::SP, 0xFFFE);
        self.cpu.set_r16(&R::BP, 0x091C); // is what dosbox used

        // This is what dosbox initializes the registers to
        // at program load
        self.cpu.set_r16(&R::CX, 0x00FF);
        self.cpu.set_r16(&R::DX, psp_segment);
        self.cpu.set_r16(&R::SI, 0x0100); // XXX 0 on .exe load
        self.cpu.set_r16(&R::DI, 0xFFFE); // XXX 0x1000 on .exe

        self.cpu.regs.ip = 0x0100;
        let min = self.cpu.get_address();
        self.cpu.rom_base = min;

        let cs = self.cpu.get_r16(&R::CS);
        self.hw.mmu.write(cs, self.cpu.regs.ip, data);
    }

    // returns a copy of register values at a given time
    pub fn register_snapshot(&self) -> RegisterSnapshot {
        self.cpu.regs.clone()
    }

    // executes enough instructions that can run for 1 video frame
    pub fn execute_frame(&mut self) {
        let fps = 60;
        let cycles = self.cpu.clock_hz / fps;
        // println!("will execute {} cycles", cycles);

        loop {
            self.execute_instruction();
            if self.cpu.fatal_error {
                break;
            }
            if self.cpu.cycle_count > cycles {
                self.cpu.cycle_count = 0;
                break;
            }
        }
    }

    // executes n instructions of the cpu. only used in tests
    pub fn execute_instructions(&mut self, count: usize) {
        for _ in 0..count {
            self.execute_instruction()
        }
    }

    // returns first line of disassembly
    fn external_disasm_of_bytes(&self, cs: u16, ip: u16) -> String {
        let bytes = self.hw.mmu.read(cs, ip, 16);
        let s = ndisasm_bytes(&bytes).unwrap();
        let ln = s.find("\n").unwrap();
        s[0..ln].to_owned()
    }

    pub fn execute_instruction(&mut self) {
        let cs = self.cpu.get_r16(&R::CS);
        let ip = self.cpu.regs.ip;
        if cs == 0xF000 {
            // we are in interrupt vector code, execute high-level interrupt.
            // the default interrupt vector table has a IRET
            self.cpu.handle_interrupt(&mut self.hw, ip as u8);
        }

        let op = self.cpu.decoder.get_instruction(&mut self.hw.mmu, cs, ip);

        match op.command {
            Op::Unknown => {
                self.cpu.fatal_error = true;
                println!("executed unknown op, stopping. {} instructions executed",
                         self.cpu.instruction_count);
            }
            Op::Invalid(reason) => {
                self.cpu.fatal_error = true;
                match reason {
                    InvalidOp::Op => {
                        println!("[{:04X}:{:04X}] Error unhandled OP", cs, ip);
                        println!("ndisasm: {}", self.external_disasm_of_bytes(cs, ip));
                    }
                    InvalidOp::Reg(reg) => {
                        println!("[{:04X}:{:04X}] Error invalid register {:02X}", cs, ip, reg);
                        println!("ndisasm: {}", self.external_disasm_of_bytes(cs, ip));
                    }
                    InvalidOp::Byte(b) => {
                        println!("[{:04X}:{:04X}] Error invalid byte {:02X}", cs, ip, b);
                        println!("ndisasm: {}", self.external_disasm_of_bytes(cs, ip));
                    }
                }
                println!("{} Instructions executed", self.cpu.instruction_count);
            }
            _ => self.cpu.execute(&mut self.hw, &op),
        }

        // XXX need instruction timing to do this properly
        if self.cpu.cycle_count % 100 == 0 {
            self.hw.gpu.progress_scanline();
        }

        if self.cpu.cycle_count % 100 == 0 {
            // FIXME: counter should decrement ~18.2 times/sec
            self.hw.pit.counter0.dec();
        }
    }
}
