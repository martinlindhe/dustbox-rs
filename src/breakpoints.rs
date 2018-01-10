
#[derive(Default)]
pub struct Breakpoints {
    breakpoints: Vec<usize>,
}

impl Breakpoints {
     pub fn new() -> Self {
        Breakpoints {
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

    pub fn get(&self) -> Vec<usize> {
        // XXX return sorted list .sort()
        self.breakpoints.clone()
    }

    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }

    // returns true if offset is at breakpoint
    pub fn hit(&self, offset: usize) -> bool {
        self.breakpoints.iter().any(|&x| x == offset)
    }
}
