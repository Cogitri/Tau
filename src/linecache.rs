use std::collections::HashMap;
use std::cmp::min;

use error::*;
use structs::*;

#[derive(Debug, Clone)]
pub struct Line {
    pub cursor: Vec<usize>,
    pub text: String,
    pub styles: Vec<usize>,
}

#[derive(Debug)]
pub struct LineCache {
    map: HashMap<u64, Line>,
    n_invalid_before: u64,
    lines: Vec<Option<Line>>,
    n_invalid_after: u64,
}

impl LineCache {
    pub fn new() -> LineCache {
        LineCache {
            map: HashMap::new(),
            n_invalid_before: 0,
            lines: Vec::new(),
            n_invalid_after: 0,
        }
    }
    pub fn len(&self) -> u64 {
        self.n_invalid_before + self.lines.len() as u64 + self.n_invalid_after
    }
    pub fn get(&self, n: u64) -> Option<&Line> {
        if n < self.n_invalid_before
            || n > self.n_invalid_before + self.lines.len() as u64 {
                return None;
        }
        let ix = (n - self.n_invalid_before) as usize;
        if let Some(&Some(ref line)) = self.lines.get(ix) {
            Some(line)
        } else {
            None
        }
    }
    pub fn get_missing(&self, first: u64, last: u64) -> Vec<(u64, u64)> {
        let mut ret = Vec::new();
        let last = min(last, self.len());
        assert!(first < last);

        let mut run = None;
        for ix in first..last {
            if ix < self.n_invalid_before
                || ix >= self.n_invalid_before + self.lines.len() as u64
                || self.lines[(ix - self.n_invalid_before) as usize].is_none() {

                match run {
                    None => {run = Some((ix, ix+1));}
                    Some((f,l)) if l == ix => {run = Some((f, ix + 1));}
                    Some((f,l)) => {
                        ret.push((f,l));
                        run = Some((ix, ix + 1));
                    }
                }
            }
        }
        if let Some((f,l)) = run {
            ret.push((f,l));
        }
        ret
    }
    pub fn handle_update(&mut self, ops: &[UpdateOp]) -> Result<(), GxiError> {
        let mut new_invalid_before = 0;
        let mut new_lines: Vec<Option<Line>> = Vec::new();
        let mut new_invalid_after = 0;

        for op in ops {
            let ref op_type = op.op;
            debug!("lc before {}-- {:?}", op_type, self);
            let mut idx = 0u64;
            let mut n = op.n;
            let mut old_ix = 0u64;
            match op_type.as_ref() {
                "invalidate" => {
                    if new_lines.len() == 0 {
                        new_invalid_before += n;
                    } else {
                        new_invalid_after += n;
                    }
                },
                "ins" => {
                    for _ in 0..new_invalid_after {
                        new_lines.push(None);
                    }
                    new_invalid_after = 0;
                    //let json_lines = op.lines.unwrap_or_else(Vec::new);
                    for json_line in op.lines.iter().flat_map(|l| l.iter()) {
                        new_lines.push(Some(Line{
                            cursor: json_line.cursor.clone().unwrap_or_else(Vec::new),
                            text: json_line.text.clone(),
                            styles: json_line.styles.clone().unwrap_or_else(Vec::new),
                        }));
                    }
                },
                "copy" | "update" => {
                    let mut n_remaining = n;
                    if old_ix < self.n_invalid_before {
                        let n_invalid = min(n, self.n_invalid_before - old_ix);
                        if new_lines.len() == 0 {
                            new_invalid_before += n_invalid;
                        } else {
                            new_invalid_after += n_invalid;
                        }
                        old_ix += n_invalid;
                        n_remaining -= n_invalid;
                    }
                    if n_remaining > 0 && old_ix < self.n_invalid_before + self.lines.len() as u64 {
                        let n_copy = min(n_remaining, self.n_invalid_before + self.lines.len() as u64 - old_ix);
                        let start_ix = old_ix - self.n_invalid_before;
                        if op_type == "copy" {
                            new_lines.extend_from_slice(&mut self.lines[start_ix as usize .. (start_ix + n_copy) as usize]);
                        } else {
                            if let Some(ref json_lines) = op.lines {
                                //let json_lines = op.lines.unwrap_or_else(Vec::new);
                                let mut json_ix = n - n_remaining;
                                for ix in start_ix .. start_ix + n_copy {
                                    if let Some(&Some(ref json_line)) = self.lines.get(ix as usize) {
                                        let mut new_line = json_line.clone();
                                        if let Some(ref json_line) = json_lines.get(json_ix as usize) {
                                            new_line.text = json_line.text.clone();
                                            if let Some(ref cursor) = json_line.cursor {
                                                new_line.cursor = cursor.clone();
                                            }
                                            if let Some(ref styles) = json_line.styles {
                                                new_line.styles = styles.clone();
                                            }
                                        }
                                        new_lines.push(Some(new_line));
                                    }
                                    json_ix += 1;
                                }
                            }
                        }
                        old_ix += n_copy;
                        n_remaining -= n_copy;
                    }
                    if new_lines.len() == 0 {
                        new_invalid_before += n_remaining;
                    } else {
                        new_invalid_after += n_remaining;
                    }
                    old_ix += n_remaining;
                },
                "skip" => {
                    old_ix += n;
                },
                _ => {

                },
            }
        }
        self.n_invalid_before = new_invalid_before;
        self.lines = new_lines;
        self.n_invalid_after = new_invalid_after;
        debug!("lc after update {:?}", self);
        Ok(())
    }
}
