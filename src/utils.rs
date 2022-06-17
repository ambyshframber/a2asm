use std::iter::Iterator;
use thiserror::Error;
//use std::ops::Deref;
//use std::fmt;

pub struct Tokeniser<'a> {
    code: &'a str,
    line: i32,
}

// scrub over string until you find a valid split point
// use a separate tokeniser iterator to allow easy changes later
impl Tokeniser<'_> {
    pub fn new(code: &str) -> Tokeniser {
        Tokeniser {
            code,
            line: 0
        }
    }
}

impl<'a> Iterator for Tokeniser<'a> {
    type Item = Result<Token<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.code.trim() == "" {
            return None
        }
        let mut hit_word = false;
        let mut hit_comment = false;
        let mut in_line_comment = false;
        let mut in_multiline_comment = false;
        let mut tstart = 0;
        let mut line_at_tstart = 0;
        let mut bracket_layers = 0;
        
        for (i, c) in self.code.char_indices() {
            //println!("{:?}, hc {}, slc {}, mlc {}", c, hit_comment, in_line_comment, in_multiline_comment);
            if c == '\n' { // ALWAYS count lines
                self.line += 1
            }
            if hit_comment && !in_multiline_comment { // iterate until matching `\` or newline
                if c == '/' {
                    in_line_comment = true
                }
                else if c == '*' {
                    in_multiline_comment = true
                }
                hit_comment = false
            }
            else if in_line_comment {
                if c == '\n' {
                    in_line_comment = false
                }
            }
            else if in_multiline_comment {
                if c == '*' {
                    hit_comment = true // reuse variable for exiting comments
                }
                else if hit_comment {
                    //println!("maybe exit mlc");
                    //println!("{:?}", c);
                    if c == '/' {
                        hit_comment = false;
                        in_multiline_comment = false
                    }
                }
                else {
                    //println!("do not exit mlc");
                    hit_comment = false
                }
            }
            else {
                if c == '(' { // only count brackets when outside comments
                    bracket_layers += 1
                }
                if c == ')' {
                    bracket_layers -= 1
                }
                //println!("{:?} {}", c, bracket_layers);
                if !hit_word { // iterate until word start
                    if c.is_whitespace() { // keep going
                        continue
                    }
                    else if c == '/' { // enter comment
                        hit_comment = true
                    }
                    else { // not whitespace or comment
                        tstart = i; // remember where token starts
                        line_at_tstart = self.line;
                        hit_word = true
                    }
                }
                else { // found word
                    if (c.is_whitespace()) && bracket_layers == 0 { // end word
                        if c == '\n' {
                            self.line -= 1
                        }
                        let word = &self.code[tstart..i].trim(); // cut out token
                        let t = Token {
                            token: word,
                            line: line_at_tstart
                        };
                        self.code = &self.code[i..];
                        return Some(Ok(t))
                    }
                    else { // not whitespace or comment
                        continue
                    }
                }
            }
        }
        if in_line_comment || in_multiline_comment || !hit_word {
            self.code = "";
            return None
        }
        let t = Token {
            token: self.code.trim(),
            line: self.line
        };
        self.code = ""; // overwrite code so we don't keep returning some
        Some(Ok(t))
    }
}
#[derive(Debug, PartialEq)]
pub struct Token<'a> {
    pub token: &'a str,
    pub line: i32
}

pub fn op_to_byte(op: &str) -> Result<u8> {
    let mut has_modes = true;
    let mut has_k = true;
    let mut b = if op.len() >= 3 { // get base opcode
        let op_trim = &op[..3];
        match op_trim {
            // stack
            "POP" => 0x03, "SWP" => 0x04, "ROT" => 0x05, "DUP" => 0x06, "OVR" => 0x07,
            // logic/jumps
            "EQU" => 0x08, "GTH" => 0x09, "JMP" => 0x0a, "JNZ" => 0x0b, "JSR" => 0x0c, "STH" => 0x0d,
            // mem
            "LDZ" => 0x10, "STZ" => 0x11, "LDR" => 0x12, "STR" => 0x13, "LDA" => 0x14, "STA" => 0x15, "PIC" => 0x16, "PUT" => 0x17,
            // arithmetic
            "ADC" => 0x18, "SBC" => 0x19, "MUL" => 0x1a, "DVM" => 0x1b, "AND" => 0x1c, "IOR" => 0x1d, "XOR" => 0x1e, "SFT" => 0x1f,
            // match guard
            _ => {
                has_modes = false;
                match op_trim {
                    "LIT" => {
                        has_k = false;
                        has_modes = true;
                        0x80
                    }
                    "SEC" => 0x20,
                    "CLC" => 0x40,
                    "EXT" => 0x60,
                    "RTI" => 0x83,
                    "NOP" => 0,
                    _ => return Err(AvcErr::BadInstr(op.into()))
                }
            }
        }
    }
    else {
        return Err(AvcErr::BadInstr(op.into()))
    };

    let modes = &op[3..];
    if has_modes {
        for c in modes.chars() {
            match c {
                'k' => {
                    if has_k {
                        b |= 0x80
                    }
                    else {
                        return Err(AvcErr::BadMode(op.into(), 'k'))
                    }
                }
                'r' => b |= 0x40,
                '2' => b |= 0x20,
                _ => return Err(AvcErr::BadMode(op.into(), c))
            }
        }
    }
    else if !modes.is_empty() {
        // error here
    }

    Ok(b)
}

pub fn set_vec_at<T: Default>(v: &mut Vec<T>, val: T, idx: usize) {
    let vlen = v.len();
    if idx == vlen {
        v.push(val)
    }
    else if idx > vlen {
        v.resize_with(idx, || T::default());
        v.push(val)
    }
    else {
        v[idx] = val
    }
}

pub fn split_bracket_groups(s: &str, split: char) -> Vec<&str> {
    let mut ret = Vec::new();
    let mut last_idx = 0;
    let mut bracket_layers = 0;
    for (i, c) in s.char_indices() {
        //println!("{}", c);
        if c == '(' {
            bracket_layers += 1
        }
        else if c == ')' {
            bracket_layers -= 1
        }
        else if c == split && bracket_layers == 0 {
            // split here
            ret.push(s[last_idx..i].trim());
            last_idx = i + 1
        }
    }
    ret.push(s[last_idx..].trim());
    ret
}

pub type Result<T> = std::result::Result<T, AvcErr>;

#[derive(PartialEq, Debug, Error)]
pub enum AvcErr {
    #[error("bad instruction: {0}")]
    BadInstr(String),
    #[error("bad mode: {0} cannot have mode {1}")]
    BadMode(String, char),
    #[error("unrecognised directive: {0}")]
    UnrecognisedDirective(String),
    #[error("malformed directive: {0}")]
    MalformedDirective(String),
    #[error("undefined label: {0}")]
    UndefinedLabel(String),
    #[error("bad integer literal: {0}")]
    BadInt(String),
    #[error("byte not in code space")]
    OpNotInCodeSpace,
    #[error("relative jump too large")]
    RelJumpTooLarge,
    #[error("undefined macro: {0}")]
    UndefinedMacro(String),
    #[error("multibyte char: {0}")]
    MultibyteChar(char)
}

/*
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum MaybeOwned<'a> { // makes macros easier
    Owned(String),
    Ref(&'a str)
}
impl Deref for MaybeOwned<'_> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(s) => s.as_str(),
            Self::Ref(s) => s
        }
    }
}
impl<'a> From<String> for MaybeOwned<'a> {
    fn from(s: String) -> MaybeOwned<'a> {
        MaybeOwned::Owned(s)
    }
}
impl<'a> From<&'a str> for MaybeOwned<'a> {
    fn from(s: &'a str) -> MaybeOwned<'a> {
        MaybeOwned::Ref(s)
    }
}
impl AsRef<str> for MaybeOwned<'_> {
    fn as_ref(&self) -> &str {
        match self {
            Self::Owned(s) => s,
            Self::Ref(s) => s
        }
    }
}
impl From<MaybeOwned<'_>> for String {
    fn from(s: MaybeOwned<'_>) -> String {
        match s {
            MaybeOwned::Owned(s) => s,
            MaybeOwned::Ref(s) => String::from(s)
        }
    }
}
impl fmt::Display for MaybeOwned<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}
*/
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_o2b() {
        assert_eq!(op_to_byte("LIT"), Ok(0x80));
        assert_eq!(op_to_byte("LITr"), Ok(0xc0));
        assert_eq!(op_to_byte("RTI"), Ok(0x83));
        assert!(op_to_byte("AAA").is_err())
    }

    #[test]
    fn test_tokenise() {
        let mut t = Tokeniser::new("one two\nthree  four\t five");
        assert_eq!(t.next(), Some(Ok(Token { token: "one", line: 0 })));
        assert_eq!(t.next(), Some(Ok(Token { token: "two", line: 0 })));
        assert_eq!(t.next(), Some(Ok(Token { token: "three", line: 1 })));
        assert_eq!(t.next(), Some(Ok(Token { token: "four", line: 1 })));
        assert_eq!(t.next(), Some(Ok(Token { token: "five", line: 1 })));
        assert_eq!(t.next(), None);
    }
    #[test]
    fn adv_tokenise() {
        let mut t = Tokeniser::new("test test(aa\n bb cc)\nnext");
        assert_eq!(t.next(), Some(Ok(Token { token: "test", line: 0 })));
        assert_eq!(t.next(), Some(Ok(Token { token: "test(aa\n bb cc)", line: 0 })));
        assert_eq!(t.next(), Some(Ok(Token { token: "next", line: 2 })));
        assert_eq!(t.next(), None);
    }
    #[test]
    fn comments() {
        let mut t = Tokeniser::new(r"
//comment
test
/* comment */ test(aa //a
 bb cc)
// comment
next //end");
        assert_eq!(t.next(), Some(Ok(Token { token: "test", line: 2 })));
        assert_eq!(t.next(), Some(Ok(Token { token: "test(aa //a\n bb cc)", line: 3 })));
        assert_eq!(t.next(), Some(Ok(Token { token: "next", line: 6 })));
        assert_eq!(t.next(), None);
    }

    #[test]
    fn sbg() {
        assert_eq!(
            split_bracket_groups("abc, (123, 456), xyz", ','),
            vec!["abc", "(123, 456)", "xyz"]
        )
    }
}
