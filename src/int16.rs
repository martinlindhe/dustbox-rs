use cpu::CPU;
use register::AX;

// keyboard related interrupts
pub fn handle(cpu: &mut CPU) {
    match cpu.r16[AX].hi_u8() {
        0x00 => {
            // KEYBOARD - GET KEYSTROKE
            // Return:
            // AH = BIOS scan code
            // AL = ASCII character
            cpu.r16[AX].val = 0; // XXX
            println!("XXX impl KEYBOARD - GET KEYSTROKE");
        }
        _ => {
            println!("int16 error: unknown AH={:02X}, AX={:04X}",
                     cpu.r16[AX].hi_u8(),
                     cpu.r16[AX].val);
        }
    }
}
