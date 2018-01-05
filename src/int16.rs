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
        0x01 => {
            // KEYBOARD - CHECK FOR KEYSTROKE
            // Return:
            // ZF set if no keystroke available
            // ZF clear if keystroke available
            // AH = BIOS scan code
            // AL = ASCII character

            println!("XXX impl KEYBOARD - CHECK FOR KEYSTROKE");
            cpu.flags.zero = true;
        }
        0x11 => {
            // KEYBOARD - CHECK FOR ENHANCED KEYSTROKE (enh kbd support only)
            // Return:
            // ZF set if no keystroke available
            // ZF clear if keystroke available
            // AH = BIOS scan code
            // AL = ASCII character
            println!("XXX impl KEYBOARD - CHECK FOR ENHANCED KEYSTROKE");
            cpu.flags.zero = true;
        }
        _ => {
            println!("int16 error: unknown AH={:02X}, AX={:04X}",
                     cpu.r16[AX].hi_u8(),
                     cpu.r16[AX].val);
        }
    }
}
