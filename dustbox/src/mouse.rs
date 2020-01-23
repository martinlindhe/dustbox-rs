/// PS/2 Mouse implementation
/// Exposes a 2D mouse pointer with a left, right and middle buttons
///
/// https://wiki.osdev.org/Mouse_Input

use crate::cpu::{CPU, R};
use crate::machine::Component;
use crate::memory::MMU;

const DEBUG_MOUSE: bool = true;

#[derive(Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub struct Mouse {
    x: i32,
    y: i32,
    left: bool,
    right: bool,
    middle: bool,
}

impl Component for Mouse {
    fn int(&mut self, int: u8, cpu: &mut CPU, _mmu: &mut MMU) -> bool {
        if int != 0x33 {
            return false;
        }
        match cpu.get_r16(R::AX) {
            0x0003 => {
                // MS MOUSE v1.0+ - RETURN POSITION AND BUTTON STATUS
                // Return:
                // BX = button status
                // CX = column
                // DX = row
                cpu.set_r16(R::BX, self.button_status());
                cpu.set_r16(R::CX, (self.x*2) as u16); // XXX works in mode 0x13 but why multiply
                cpu.set_r16(R::DX, self.y as u16);

                // XXX Note: In text modes, all coordinates are specified as multiples of the cell size, typically 8x8 pixels

                if DEBUG_MOUSE {
                    println!("MOUSE - RETURN POSITION AND BUTTON STATUS");
                }
            }
            _ => return false
        }
        true
    }
}

impl Mouse {
    pub fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            left: false,
            right: false,
            middle: false,
        }
    }

    /// Sets the mouse absolute position
    pub fn set_position(&mut self, x: i32, y: i32) {
        if DEBUG_MOUSE {
            println!("mouse.set_position {}, {}", x, y);
        }
        self.x = x;
        self.y = y;
    }

    /// Sets the mouse button pressed state
    pub fn set_button(&mut self, button: MouseButton, pressed: bool) {
        if DEBUG_MOUSE {
            println!("mouse.set_button {:?}, {}", button, pressed);
        }
        match button {
            MouseButton::Left => self.left = pressed,
            MouseButton::Right => self.right = pressed,
            MouseButton::Middle => self.middle = pressed,
        }
    }

    /// returns the button status bitmask, used by INT 33, ax=03
    fn button_status(&self) -> u16 {
        let mut v: u16 = 0;
        if self.left {
            v |= 0b001;
        } else {
            v &= 0b110;
        }
        if self.right {
            v |= 0b010;
        } else {
            v &= 0b101;
        }
        if self.middle {
            v |= 0b100;
        } else {
            v &= 0b011;
        }
        v
    }
}
