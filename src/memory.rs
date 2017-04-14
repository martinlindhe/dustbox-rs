pub struct Memory {
    pub memory: Vec<u8>,
}

impl Memory {
    pub fn new() -> Memory {
        Memory { memory: vec![0u8; 0x10000 * 64] }
    }
}
