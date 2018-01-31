use cpu::CPU;
use cpu::register::{CS, AX};

// mouse related interrupts
pub fn handle(cpu: &mut CPU) {
    match cpu.r16[AX].val {
        0x0003 => {
            // MS MOUSE v1.0+ - RETURN POSITION AND BUTTON STATUS
            // Return:
            // BX = button status (see #03168)
            // CX = column
            // DX = row
            // Note: In text modes, all coordinates are specified as multiples of the cell size, typically 8x8 pixels 
            println!("XXX impl MOUSE - RETURN POSITION AND BUTTON STATUS");
        }
        _ => {
            println!("int33 error: unknown AX={:04X}. ip={:04X}:{:04X}",
                     cpu.r16[AX].val,
                     cpu.sreg16[CS],
                     cpu.ip);
        }
    }
}
