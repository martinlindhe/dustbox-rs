use cpu::CPU;
use gpu::GPU;
use cpu::op::{Op, InvalidOp};
use cpu::register::{SR, R16, RegisterSnapshot};
use cpu::segment::Segment;
use memory::mmu::MMU;
use gpu::font::load_fonts;
use hardware::Hardware;

pub struct Machine {
    pub hw: Hardware,
    pub cpu: CPU,
}

impl Machine {
    pub fn new() -> Self {
        Machine {
            cpu: CPU::new(),
            hw: Hardware::new(),
        }
    }

   // reset the CPU and memory
    pub fn hard_reset(&mut self) {
        self.cpu = CPU::new();
    }

    // load .com program into CS:0100 and set IP to program start
    pub fn load_com(&mut self, data: &[u8]) {
        // CS,DS,ES,SS = PSP segment
        let psp_segment = 0x085F; // is what dosbox used
        self.cpu.set_sr(&SR::CS, psp_segment);
        self.cpu.set_sr(&SR::DS, psp_segment);
        self.cpu.set_sr(&SR::ES, psp_segment);
        self.cpu.set_sr(&SR::SS, psp_segment);

        // offset of last word available in first 64k segment
        self.cpu.set_r16(&R16::SP, 0xFFFE);
        self.cpu.set_r16(&R16::BP, 0x091C); // is what dosbox used

        // This is what dosbox initializes the registers to
        // at program load
        self.cpu.set_r16(&R16::CX, 0x00FF);
        self.cpu.set_r16(&R16::DX, psp_segment);
        self.cpu.set_r16(&R16::SI, 0x0100);
        self.cpu.set_r16(&R16::DI, 0xFFFE);

        self.cpu.ip = 0x0100;
        let min = self.cpu.get_address();
        self.cpu.rom_base = min;

        let cs = self.cpu.get_sr(&SR::CS);
        self.hw.mmu.write(cs, self.cpu.ip, data);
    }

    // returns a copy of register values at a given time
    pub fn register_snapshot(&self) -> RegisterSnapshot {
        RegisterSnapshot {
            ip: self.cpu.ip,
            r16: self.cpu.r16,
            sreg16: self.cpu.sreg16,
            flags: self.cpu.flags,
        }
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

    pub fn execute_instruction(&mut self) {
        let cs = self.cpu.get_sr(&SR::CS);
        let ip = self.cpu.ip;
        let (op, length) = self.cpu.decoder.get_instruction(&mut self.hw.mmu, Segment::DS, cs, ip);

        match op.command {
            Op::Unknown() => {
                self.cpu.fatal_error = true;
                println!("executed unknown op, stopping. {} instructions executed",
                         self.cpu.instruction_count);
            }
            Op::Invalid(reason) => {
                self.cpu.fatal_error = true;
                match reason {
                    InvalidOp::Op => {
                        let mut ops_str = Vec::new();
                        for i in 0..16 {
                            let x = self.hw.mmu.read_u8(cs, ip + i);
                            let hex = format!("0x{:02X}", x);
                            ops_str.push(hex);
                        }
                        println!("Error unhandled OP {} at {:04X}:{:04X}", ops_str.join(", "), cs, ip);
                    }
                    InvalidOp::Reg(reg) => {
                        println!("Error invalid register {:02X} at {:04X}:{:04X}", reg, cs, ip);
                    }
                }
                println!("{} Instructions executed", self.cpu.instruction_count);
            }
            _ => self.cpu.execute(&mut self.hw, &op, length),
        }

        // XXX need instruction timing to do this properly
        if self.cpu.cycle_count % 100 == 0 {
            self.hw.gpu.progress_scanline();
        }

        if self.cpu.cycle_count % 100 == 0 {
            // FIXME: counter should decrement ~18.2 times/sec
            self.cpu.pit.counter0.dec();
        }
    }

}
