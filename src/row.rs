use std::{cmp, slice::SliceIndex};
use unicode_segmentation::UnicodeSegmentation;

use crate::Position;

#[derive(Default)]
pub struct Row {
    string: String,
    len: usize,
}

impl From<&str> for Row {
    fn from(slice: &str) -> Self {
        Self{
            string: String::from(slice),
            len: slice.graphemes(true).count(),
        }
    }
}

impl Row {
    pub fn render(&self, start: usize, end: usize) -> String {
        let end = cmp::min(end, self.string.len());
        let start = cmp::min(start, end);
        let mut result = String::new();
        #[allow(clippy::integer_arithmetic)]
        for grapheme in self.string[..]
            .graphemes(true)
            .skip(start)
            .take(end - start)
        {
            if grapheme == "\t" {
                result.push_str(" ");
            } else {
                result.push_str(grapheme);
            }
        }
        result
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(&mut self, at: usize, c: char) {
        if at >= self.len() {
            self.string.push(c);
            self.len += 1;
            return;
        }
        let mut result: String = String::new();
        let mut lenght = 0;
        for (idx, grapheme) in self.string[..].graphemes(true).enumerate() {
            lenght += 1;
            if idx == at {
                lenght += 1;
                result.push(c);
            }
            result.push_str(grapheme);
        }
        self.len = lenght;
        self.string = result;
    }

    #[allow(clippy::integer_arithmetic)]
    pub fn delete(&mut self, at: usize) {
        if at >= self.len() {
            return;
        }
        let mut result: String = String::new();
        let mut lenght = 0;
        for (idx, grapheme) in self.string[..].graphemes(true).enumerate() {
            if idx != at {
                lenght += 1;
                result.push_str(grapheme);
            }
        }
        self.len = lenght;
        self.string = result;
    }

    pub fn append(&mut self, new: &Self) {
        self.string = format!("{}{}", self.string, new.string);
        self.len += new.len();
    }

    pub fn split(&mut self, at: usize) -> Self {
        let mut row: String = String::new();
        let mut lenght = 0;
        let mut splitted_row: String = String::new();
        let mut splitted_len = 0;
        for (idx, grapheme) in self.string[..].graphemes(true).enumerate() {
            if idx < at {
                lenght += 1;
                row.push_str(grapheme);
            } else {
                splitted_len += 1;
                splitted_row.push_str(grapheme);
            }
        }
        self.string = row;
        self.len = lenght;
        Self {
            string: splitted_row,
            len: splitted_len,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.string.as_bytes()
    }

    pub fn find(&self, query: &str) -> Option<usize> {
        let matching_byte_idx = self.string.find(query);
        if let Some(matching_byte_idx) = matching_byte_idx {
            for (grapheme_idx, (byte_idx, _)) in self.string[..].grapheme_indices(true).enumerate() {
                if matching_byte_idx == byte_idx {
                    return Some(grapheme_idx);
                }
            }
        }
        None
    }
}
