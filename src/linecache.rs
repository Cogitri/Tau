use log::{error, trace};
use serde_json::{json, Value};
use std::cmp::min;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug)]
pub struct StyleSpan {
    pub start: i64,
    pub len: usize,
    pub id: usize,
}

/// A Struct representing _one_ line which xi has sent us.
/// # Fields:
/// * text: Contains the text of that line
/// * line_num: The number of the line. Multiple lines may have the same num due to word wrapping.
/// * cursor: What position the cursor is at
/// * styles: What style this is (e.g. italic, underlined)
#[derive(Clone, Debug)]
pub struct Line {
    text: String,
    cursor: Vec<u64>,
    pub styles: Vec<StyleSpan>,
    line_num: u64,
}

impl Line {
    pub fn from_json(v: &Value, line_num: u64) -> Line {
        let text = v["text"].as_str().unwrap().to_owned();
        let cursor = if let Some(arr) = v["cursor"].as_array() {
            arr.iter().map(|c| c.as_u64().unwrap()).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let mut styles = Vec::new();
        if let Some(arr) = v["styles"].as_array() {
            // Convert style triples into a `Vec` of `StyleSpan`s
            let mut i = 0;
            let mut style_span = StyleSpan {
                start: 0,
                len: 0,
                id: 0,
            };
            for c in arr {
                if i == 3 {
                    i = 0;
                    styles.push(style_span);
                }
                match i {
                    0 => style_span.start = c.as_i64().unwrap() as i64,
                    1 => style_span.len = c.as_u64().unwrap() as usize,
                    2 => style_span.id = c.as_u64().unwrap() as usize,
                    _ => unreachable!(),
                };
                i += 1;
            }
            if i == 3 {
                styles.push(style_span);
            }
        }
        Line {
            text,
            cursor,
            styles,
            line_num,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> &[u64] {
        &self.cursor
    }

    pub fn line_num(&self) -> &u64 {
        &self.line_num
    }
}

#[derive(Debug)]
pub struct LineCache {
    map: HashMap<u64, Line>,
    pub n_invalid_before: u64,
    pub lines: Vec<Option<Line>>,
    pub n_invalid_after: u64,
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
    pub fn height(&self) -> u64 {
        self.n_invalid_before + self.lines.len() as u64 + self.n_invalid_after
    }
    pub fn width(&self) -> usize {
        self.lines
            .iter()
            .map(|l| match *l {
                None => 0,
                Some(ref l) => l.text.len(),
            })
            .max()
            .unwrap_or(0)
    }
    pub fn get_line(&self, n: u64) -> Option<&Line> {
        if n < self.n_invalid_before || n > self.n_invalid_before + self.lines.len() as u64 {
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
        let last = min(last, self.height());
        assert!(first < last);

        let mut run = None;
        for ix in first..last {
            if ix < self.n_invalid_before
                || ix >= self.n_invalid_before + self.lines.len() as u64
                || self.lines[(ix - self.n_invalid_before) as usize].is_none()
            {
                match run {
                    None => {
                        run = Some((ix, ix + 1));
                    }
                    Some((f, l)) if l == ix => {
                        run = Some((f, ix + 1));
                    }
                    Some((f, l)) => {
                        ret.push((f, l));
                        run = Some((ix, ix + 1));
                    }
                }
            }
        }
        if let Some((f, l)) = run {
            ret.push((f, l));
        }
        ret
    }
    pub fn apply_update(&mut self, update: &Value) {
        let mut new_invalid_before = 0;
        let mut new_lines: Vec<Option<Line>> = Vec::new();
        let mut new_invalid_after = 0;

        let mut old_ix = 0u64;

        for op in update["ops"].as_array().unwrap() {
            let op_type = &op["op"];
            //debug!("lc before {}-- {} {:?} {}", op_type, new_invalid_before, new_lines, new_invalid_after);
            let n = op["n"].as_u64().unwrap();
            match op_type.as_str().unwrap() {
                "invalidate" => {
                    trace!("invalidate n={}", n);
                    if new_lines.is_empty() {
                        new_invalid_before += n;
                    } else {
                        new_invalid_after += n;
                    }
                }
                "ins" => {
                    trace!("ins n={}", n);
                    for _ in 0..new_invalid_after {
                        new_lines.push(None);
                    }
                    new_invalid_after = 0;
                    for line in op["lines"].as_array().unwrap() {
                        // xi only send 'ln' for actual lines
                        let n = if let Some(ln) = line["ln"].as_u64() {
                            ln
                        // If it doesn't send ln this line is the result of a linebreak, so it should
                        // use the line num of the previous line.
                        } else {
                            if let Some(previous_line) = new_lines.last().cloned() {
                                previous_line
                                    .unwrap_or(Line::from_json(&json!({"text": ""}), 0))
                                    .line_num
                            } else {
                                0
                            }
                        };
                        let line = Line::from_json(line, n);
                        new_lines.push(Some(Line {
                            cursor: line.cursor.clone(),
                            text: line.text.clone(),
                            styles: line.styles.clone(),
                            line_num: line.line_num,
                        }));
                    }
                }
                "copy" => {
                    trace!("copy n={}", n);

                    for _ in 0..new_invalid_after {
                        new_lines.push(None);
                    }
                    new_invalid_after = 0;

                    let mut n_remaining = n;
                    if old_ix < self.n_invalid_before {
                        let n_invalid = min(n, self.n_invalid_before - old_ix);
                        if new_lines.is_empty() {
                            new_invalid_before += n_invalid;
                        } else {
                            new_invalid_after += n_invalid;
                        }
                        old_ix += n_invalid;
                        n_remaining -= n_invalid;
                    }
                    if n_remaining > 0 && old_ix < self.n_invalid_before + self.lines.len() as u64 {
                        let n_copy = min(
                            n_remaining,
                            self.n_invalid_before + self.lines.len() as u64 - old_ix,
                        );
                        let start_ix = old_ix - self.n_invalid_before;

                        for i in start_ix as usize..(start_ix + n_copy) as usize {
                            if self.lines[i].is_none() {
                                error!(
                                    "line {}+{}={}, a copy source is none",
                                    self.n_invalid_before,
                                    i,
                                    self.n_invalid_before + i as u64
                                );
                            }
                        }
                        new_lines.extend_from_slice(
                            &self.lines[start_ix as usize..(start_ix + n_copy) as usize],
                        );

                        old_ix += n_copy;
                        n_remaining -= n_copy;
                    }
                    if new_lines.is_empty() {
                        new_invalid_before += n_remaining;
                    } else {
                        new_invalid_after += n_remaining;
                    }
                    old_ix += n_remaining;
                }
                "skip" => {
                    trace!("skip n={}", n);
                    old_ix += n;
                }
                _ => {}
            }
        }
        self.n_invalid_before = new_invalid_before;
        self.lines = new_lines;
        self.n_invalid_after = new_invalid_after;
        //debug!("lc after update {:?}", self);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test() {
        let mut linecache = LineCache::new();
        linecache.apply_update(&json!({
            "ops": [
                {"op":"invalidate", "n": 20},
                {"op":"ins", "n": 30, "lines": [
                    {"text": "20\n", "ln": 1},
                    {"text": "21\n"},
                ]},
                {"op":"invalidate", "n": 10},
                {"op":"ins", "n": 30, "lines": [
                    {"text": "32\n"},
                    {"text": "33\n"},
                ]},
                {"op":"invalidate", "n": 10},
                {"op":"ins", "n": 30, "lines": [
                    {"text": "44\n"},
                    {"text": "45\n"},
                    {"text": "46\n"},
                    {"text": "47\n"},
                ]},
                {"op":"ins", "n": 30, "lines": [
                    {"text": "48\n"},
                    {"text": "49\n"},
                    {"text": "50\n"},
                    {"text": "51\n"},
                    {"text": "52\n"},
                ]},
                {"op":"invalidate", "n": 10},
            ]
        }));

        linecache.apply_update(&json!({
            "ops": [
                {"n":10,"op":"invalidate"},
                {"n":10,"op":"invalidate"},
                {"n":20,"op":"skip"},
                {"n":2,"op":"copy"},
                {"n":10,"op":"invalidate"},
                {"n":10,"op":"skip"},
                {"n":3,"op":"copy"},
                {"n":10,"op":"invalidate"},
                {"n":10,"op":"skip"},
                {"n":4,"op":"copy"},
                {"n":5,"op":"copy"},
                {"n":2,"op":"ins",
                    "lines":[
                        {"styles":[0,70,2],"text":"53\n"},
                        {"styles":[0,72,2],"text":"54\n"},
                ]},
                {"n":8,"op":"invalidate"},
            ]
        }));

        assert_eq!(linecache.n_invalid_before, 20);
        assert_eq!(linecache.n_invalid_after, 8);
        assert_eq!(linecache.get_line(20).unwrap().text(), "20\n");
        assert_eq!(linecache.get_line(21).unwrap().text(), "21\n");
        assert!(linecache.get_line(22).is_none());
        assert_eq!(linecache.get_line(32).unwrap().text(), "32\n");
        assert_eq!(linecache.get_line(52).unwrap().text(), "52\n");
        assert!(linecache.get_line(53).is_none());

        println!("LINE CACHE: {:?}", linecache);
    }
}
