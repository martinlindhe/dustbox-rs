pub struct GPU {
    pub scanline: u16,
}
impl GPU {
    pub fn new() -> GPU {
        GPU { scanline: 0 }
    }
    pub fn progress_scanline(&mut self) {
        // HACK to have a source of info to toggle CGA status register
        self.scanline += 1;
        if self.scanline > 100 {
            self.scanline = 0;
        }
    }
}
