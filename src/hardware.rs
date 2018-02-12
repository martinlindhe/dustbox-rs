use gpu::GPU;
use gpu::font::load_fonts;
use memory::mmu::MMU;

pub struct Hardware {
    pub gpu: GPU,
    pub mmu: MMU,
}

impl Hardware {
    pub fn new() -> Self {
        let mut mmu = MMU::new();
        load_fonts(&mut mmu);
        Hardware {
            gpu: GPU::new(),
            mmu: mmu,
        }
    }
}
