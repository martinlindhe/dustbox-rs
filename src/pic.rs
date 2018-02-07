// Programmable Interrupt Controller (8259A)

#[derive(Clone, Default)]
pub struct PIC {
    v0021: u8,
}

impl PIC {
    pub fn new() -> Self {
        PIC {
            v0021: 0,
        }
    }

    pub fn read_ocw1(&self) -> u8 {
        // read: PIC master interrupt mask register OCW1 
        0 // XXX
    }

    pub fn write_0021(&mut self, val: u8) {
        // XXX: one value if written immediately after value to 0020, another otherwise....
        self.v0021 = val;
    }
}
