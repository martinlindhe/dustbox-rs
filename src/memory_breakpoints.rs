#[cfg(test)]
#[path = "./breakpoints_test.rs"]
mod breakpoints_test;

#[derive(Default)]
pub struct MemoryBreakpoints {
    breakpoints: Vec<usize>,
}

// a list of addresses for the debugger to break on when memory content changes
impl MemoryBreakpoints {
     pub fn new() -> Self {
        MemoryBreakpoints {
            breakpoints: vec![0; 0],
        }
    }

    pub fn add(&mut self, bp: usize) -> Option<usize> {
        if self.breakpoints.iter().find(|&&x|x == bp).is_none() {
            self.breakpoints.push(bp);
            Some(bp)
        } else {
            None
        }
    }

    pub fn remove(&mut self, bp: usize) -> Option<usize> {
        // TODO later: simplify when https://github.com/rust-lang/rust/issues/40062 is stable
        match self.breakpoints.iter().position(|x| *x == bp) {
            Some(pos) => {
                self.breakpoints.remove(pos);
                Some(bp)
            },
            None => None,
        }
    }

    // returns a Vec with breakpoints sorted ascending
    pub fn get(&self) -> Vec<usize> {
        let mut sorted = self.breakpoints.clone();
        sorted.sort();
        sorted
    }

    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }

    // returns true memory value has changed since last check
    pub fn has_changed(&self, address: usize, val: u8) -> bool {
        // xxx compare to previous value
        false
    }
}
