use std::collections::HashMap;

#[cfg(test)]
#[path = "./memory_breakpoints_test.rs"]
mod memory_breakpoints_test;

#[derive(Default)]
pub struct MemoryBreakpoints {
    breakpoints: Vec<u32>,

    /// tracks previous memory values to find changes
    map: HashMap<u32, u8>,
}

/// a list of addresses for the debugger to break on when memory content changes
impl MemoryBreakpoints {
     pub fn default() -> Self {
        MemoryBreakpoints {
            breakpoints: vec![0; 0],
            map: HashMap::new(),
        }
    }

    pub fn add(&mut self, bp: u32) -> Option<u32> {
        if self.breakpoints.iter().find(|&&x|x == bp).is_none() {
            self.breakpoints.push(bp);
            Some(bp)
        } else {
            None
        }
    }

    pub fn remove(&mut self, bp: u32) -> Option<u32> {
        // TODO later: simplify when https://github.com/rust-lang/rust/issues/40062 is stable
        match self.breakpoints.iter().position(|x| *x == bp) {
            Some(pos) => {
                self.breakpoints.remove(pos);
                Some(bp)
            },
            None => None,
        }
    }

    /// returns a Vec with breakpoints sorted ascending
    pub fn get(&self) -> Vec<u32> {
        let mut sorted = self.breakpoints.clone();
        sorted.sort();
        sorted
    }

    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }

    /// returns true if memory value has changed since last check
    pub fn has_changed(&mut self, address: u32, val: u8) -> bool {
        let t = self.map.entry(address).or_insert(val);
        let old = *t;
        *t = val;
        old != val
    }
}
