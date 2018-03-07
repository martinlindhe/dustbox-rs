use hardware::Hardware;
use cpu::{CPU, R};

// mouse related interrupts
pub fn handle(cpu: &mut CPU, _hw: &mut Hardware) {
    match cpu.get_r16(&R::AX) {
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
            println!("int33 error: unknown ax={:04X}, ip={:04X}:{:04X}",
                     cpu.get_r16(&R::AX),
                     cpu.get_r16(&R::CS),
                     cpu.regs.ip);
        }
    }
}
