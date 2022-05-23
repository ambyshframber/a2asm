use std::iter::Iterator;

pub struct Tokeniser<'a> {
    code: &'a str,
    line: usize,
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
        if self.code.trim() == "" { // None if only whitespace left
            return None
        }
        let mut in_space = false;
        let mut token = self.code; // set it to the whole thing here so if there's no whitespace it returns the whole thing as one token
        let line_at_find = self.line;
        let mut bracket_layers = 0;
        let mut in_comment = false;
        for (i, c) in self.code.char_indices() { // ACTUALLY HAUNTED
            if in_comment {
                if c == '\n' || c == '\\' {
                    in_comment = false;
                    in_space = true;
                }
                continue
            }
            if c == '(' {
                bracket_layers += 1
            }
            else if c == ')' {
                bracket_layers -= 1
            }
            else if c == '\\' {
                in_comment = true;
                continue
            }
            else if c == '\n' {
                self.line += 1
            }
            if [' ', '\n', '\t'].contains(&c) { // is it a whitespace char?
                if !in_space && bracket_layers == 0 { // check we're outside brackets
                    token = &self.code[..i]; // slice out token now
                    in_space = true;
                }
            }
            else {
                if in_space { // if we've left whitespace...
                    self.code = &self.code[i..]; // shave off what we're returning + the delimiting whitespace + any comments
                    let t = Token {
                        token, line: line_at_find
                    };
                    return Some(Ok(t)) // and return the value we sliced out earlier
                }
            }
        }
        self.code = "";
        let t = Token {
            token, line: line_at_find
        };
        Some(Ok(t)) // return entire code if no whitespace
    }
}
#[derive(Debug, PartialEq)]
pub struct Token<'a> {
    pub token: &'a str,
    pub line: usize
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
                        return Err(AvcErr::BadMode('k'))
                    }
                }
                'r' => b |= 0x40,
                '2' => b |= 0x20,
                _ => return Err(AvcErr::BadMode(c))
            }
        }
    }
    else if modes.len() != 0 {
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

pub type Result<T> = std::result::Result<T, AvcErr>;

#[derive(PartialEq, Debug)]
pub enum AvcErr {
    BadInstr(String),
    BadMode(char),
    UnrecognisedDirective(String),
    MalformedDirective(String),
    UndefinedLabel(String),
    UnmatchedDelim(usize),
    BadHex(String)
}

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
}
