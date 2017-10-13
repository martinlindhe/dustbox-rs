#[derive(Clone)]
pub struct Memory {
    pub memory: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Memory { memory: vec![0u8; 0x1_0000 * 64] }
    }
}
