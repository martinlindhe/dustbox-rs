#[cfg(test)]
#[path = "./breakpoints_test.rs"]
mod breakpoints_test;

#[derive(Default)]
pub struct Breakpoints {
    breakpoints: Vec<u32>,
}

// a list of addresses for the debugger to break on when CS:IP reach one of them
impl Breakpoints {
     pub fn new() -> Self {
        Breakpoints {
            breakpoints: vec![0; 0],
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

    // returns a Vec with breakpoints sorted ascending
    pub fn get(&self) -> Vec<u32> {
        let mut sorted = self.breakpoints.clone();
        sorted.sort();
        sorted
    }

    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }

    // returns true if address is at breakpoint
    pub fn hit(&self, address: u32) -> bool {
        self.breakpoints.iter().any(|&x| x == address)
    }
}
