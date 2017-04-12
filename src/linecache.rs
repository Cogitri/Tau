use std::collections::HashMap;

#[derive(Debug)]
pub struct Line {
    pub cursor: Vec<usize>,
    pub text: String,
}

#[derive(Debug)]
pub struct LineCache {
    map: HashMap<u64, Line>,
}

impl LineCache {
    pub fn new() -> LineCache {
        LineCache {
            map: HashMap::new(),
        }
    }
    pub fn insert(&mut self, n: u64, line: Line) {
        self.map.insert(n, line);
    }
    pub fn get(&self, n: u64) -> Option<&Line> {
        self.map.get(&n)
    }
}
