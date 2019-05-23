use crate::cpu::{CPU, R};
use crate::machine::Component;

pub struct Disk {
}

impl Component for Disk {
    fn in_u8(&mut self, _port: u16) -> Option<u8> {
        None
    }

    fn out_u8(&mut self, _port: u16, _data: u8) -> bool {
        false
    }

    fn int(&mut self, int: u8, cpu: &mut CPU) -> bool {
        if int != 0x13 {
            return false;
        }
        match cpu.get_r8(R::AH) {
            0x00 => {
                // DISK - RESET DISK DRIVES
                // DL = drive (if bit 7 is set both hard disks and floppy disks reset)
                println!("XXX DISK - RESET DISK SYSTEM, dl={:02X}", cpu.get_r8(R::DL))
                // Return:
                // AH = status (see #00234)
                // CF clear if successful (returned AH=00h)
                // CF set on error
            }
            _ => return false
        }

        true
    }
}

impl Disk {
    pub fn default() -> Self {
        Self {
        }
    }
}
