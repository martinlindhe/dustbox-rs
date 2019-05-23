use crate::cpu::{CPU, R};
use crate::machine::Component;

pub struct Mouse {
}

impl Component for Mouse {
    fn in_u8(&mut self, port: u16) -> Option<u8> {
        None
    }

    fn out_u8(&mut self, port: u16, data: u8) -> bool {
        false
    }

    fn int(&mut self, int: u8, cpu: &mut CPU) -> bool {
        if int != 0x33 {
            return false;
        }
        match cpu.get_r16(R::AX) {
            0x0003 => {
                // MS MOUSE v1.0+ - RETURN POSITION AND BUTTON STATUS
                // Return:
                // BX = button status (see #03168)
                // CX = column
                // DX = row
                // Note: In text modes, all coordinates are specified as multiples of the cell size, typically 8x8 pixels 
                println!("XXX impl MOUSE - RETURN POSITION AND BUTTON STATUS");
            }
            _ => return false
        }

        true
    }
}

/// Implements a PS/2 Mouse
/// https://wiki.osdev.org/Mouse_Input
impl Mouse {
    pub fn default() -> Self {
        Self {
        }
    }
}
