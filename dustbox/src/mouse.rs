/// PS/2 Mouse implementation
/// Exposes a 2D mouse pointer with a left, right and middle buttons
///
/// https://wiki.osdev.org/Mouse_Input

use crate::cpu::{CPU, R};
use crate::machine::Component;
use crate::memory::MMU;

const DEBUG_MOUSE: bool = false;

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

    min_x: u16,
    max_x: u16,
    min_y: u16,
    max_y: u16,
}

impl Component for Mouse {
    fn int(&mut self, int: u8, cpu: &mut CPU, _mmu: &mut MMU) -> bool {
        if int != 0x33 {
            return false;
        }
        // NOTE: logitech mouse extender use AH too
        match cpu.get_r16(R::AX) {
            0x0000 => {
                // MS MOUSE - RESET DRIVER AND READ STATUS
                cpu.set_r16(R::AX, 0xFFFF); // hardware/driver installed
                cpu.set_r16(R::BX, 0x0003); // three-button mouse
            }
            0x0003 => {
                // MS MOUSE v1.0+ - RETURN POSITION AND BUTTON STATUS
                cpu.set_r16(R::BX, self.button_status());   // BX = button status
                cpu.set_r16(R::CX, self.x as u16);          // CX = column
                cpu.set_r16(R::DX, self.y as u16);          // DX = row
                if DEBUG_MOUSE {
                    println!("MOUSE - RETURN POSITION AND BUTTON STATUS");
                }
            }
            0x0007 => {
                // MS MOUSE v1.0+ - DEFINE HORIZONTAL CURSOR RANGE
                // CX = minimum column
                // DX = maximum column
                // Note: In text modes, the minimum and maximum columns are truncated to the next lower multiple of the cell size, typically 8x8 pixels 
                let cx = cpu.get_r16(R::CX);
                let dx = cpu.get_r16(R::DX);
                self.min_x = cx;
                self.max_x = dx;
                if DEBUG_MOUSE {
                    println!("MOUSE - DEFINE HORIZONTAL CURSOR RANGE min {}, max {}", cx, dx);
                }
            }
            0x0008 => {
                // MS MOUSE v1.0+ - DEFINE VERTICAL CURSOR RANGE
                // CX = minimum row
                // DX = maximum row
                // Note: In text modes, the minimum and maximum rows are truncated to the next lower multiple of the cell size, typically 8x8 pixels 
                let cx = cpu.get_r16(R::CX);
                let dx = cpu.get_r16(R::DX);
                self.min_y = cx;
                self.max_y = dx;
                if DEBUG_MOUSE {
                    println!("MOUSE - DEFINE VERTICAL CURSOR RANGE min {}, max {}", cx, dx);
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
            min_x: 0,
            max_x: 640,
            min_y: 0,
            max_y: 200,
        }
    }

    /// Sets the mouse absolute position
    pub fn set_position(&mut self, x: i32, y: i32) {
        if DEBUG_MOUSE {
            // println!("mouse.set_position {}, {}", x, y);
        }
        // XXX In text modes, all coordinates are specified as multiples of the cell size, typically 8x8 pixels

        if x >= 0 && y >= 0 {
            let screen_w = 320; // XXX
            let screen_h = 200;
            self.x = ((self.min_x + x as u16) * (self.max_x / screen_w)) as i32;
            self.y = ((self.min_y + y as u16) * (self.max_y / screen_h)) as i32;
        }
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
