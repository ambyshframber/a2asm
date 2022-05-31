use crate::utils::*;
use std::collections::HashMap;

pub struct Assembler<'a> {
    code: &'a str,
    words: Vec<Word<'a>>,
    labels: HashMap<&'a str, u16>
}

impl<'a> Assembler<'a> {
    pub fn new(code: &'a str) -> Assembler {
        Assembler {
            code,
            words: Vec::new(),
            labels: HashMap::new()
        }
    }
    pub fn assemble(&mut self) -> Result<Vec<u8>> {
        self.pass_1()?;
        println!("pass 1 completed");
        self.pass_2()?;
        println!("pass 2 completed");

        self.pass_3()
    }
    /// tokenise and parse
    fn pass_1(&mut self) -> Result<()> {
        let t = Tokeniser::new(self.code);
        self.words.push(Word::AbsPad(0x0300)); // implicit pad to default program init
        println!("tokenising...");
        for token in t {
            //println!("{}", token);
            let words = self.token_to_words(token?.token)?;
            for w in words {
                self.words.push(w)
            }
        }

        Ok(())
    }
    /// run through and calculate labels
    fn pass_2(&mut self) -> Result<()> {
        let mut counter = 0;
        for word in &self.words {
            if let Word::Lbl(l) = word {
                println!("label {} at {:04x}", l, counter);
                self.labels.insert(l, counter as u16);
            }
            if !(matches!(word, Word::Lbl(_)) || matches!(word, Word::AbsPad(_)) || matches!(word, Word::RelPad(_))) { // something byte-like
                if counter < 0x0300 { // in zpg/stack
                    println!("{:?}", word);
                    return Err(AvcErr::OpNotInCodeSpace)
                }
            }
            counter = word.next_offset(counter);
        }

        Ok(())
    }
    /// bytes!
    fn pass_3(&mut self) -> Result<Vec<u8>> {
        // rom header
        let mut ret = vec![0x41, 0x56, 0x43, 0x00];
        let mut counter = 4; // start at 4 to compensate for header
        for word in &self.words {
            //println!("{:?}", word);
            let counter_inner = if counter >= 0x0300 { // if we're in code space ie. adding bytes, shift counter down
                counter - 0x0300
            }
            else {
                counter
            };
            match word {
                Word::Byte(b) => set_vec_at(&mut ret, *b, counter_inner),
                Word::LblCall(l, k) => {
                    let addr = self.labels.get(l).ok_or(AvcErr::UndefinedLabel(String::from(*l)))?;
                    match k {
                        LblKind::Abs => {
                            // decrease by 1 to compensate for pc increasing after jump
                            // wrapping in case it underflows
                            let [hb, lb] = (addr.wrapping_sub(1)).to_be_bytes();
                            set_vec_at(&mut ret, hb, counter_inner);
                            set_vec_at(&mut ret, lb, counter_inner + 1);
                        }
                        _ => unimplemented!()
                    }
                }
                _ => {}
            }
            counter = word.next_offset(counter - 4) + 4 // add 4 to compensate for header
        }

        Ok(ret)
    }

    fn token_to_words(&mut self, s: &'a str) -> Result<Vec<Word<'a>>> { // returns a vec because macros and strings
        //println!("parsing {}", s);
        match &s[..1] { // tokens should never be 0 length
            "." => {
                self.process_directive(&s[1..])
            }
            "#" => { // raw hex
                let b = u8::from_str_radix(&s[1..], 16).map_err(|_| AvcErr::BadHex(String::from(&s[1..])))?;
                Ok(vec![Word::Byte(b)])
            }
            _ => {
                let op = op_to_byte(s)?;
                Ok(vec![Word::Byte(op)])
            }
        }
    }
    // shave off . before calling
    fn process_directive(&mut self, dir: &'a str) -> Result<Vec<Word<'a>>> {
        let (directive_name, args) = dir.split_once('(').ok_or(AvcErr::MalformedDirective(String::from(dir)))?;
        if !args.ends_with(')') { // not actually sure this can happen but it doesn't hurt to check
            return Err(AvcErr::MalformedDirective(String::from(dir)))
        }
        let args = &args[..args.len() - 1];
        let mut ret = Vec::new();
        match directive_name {
            "label" | "lbl" => {
                ret.push(Word::Lbl(args))
            }
            "absc" => {
                ret.push(Word::LblCall(args, LblKind::Abs))
            }
            "hex" | "x" => {
                let b = u8::from_str_radix(args, 16).map_err(|_| AvcErr::BadHex(String::from(args)))?;
                ret.push(Word::Byte(b))
            }
            "b" => {
                let b = u8::from_str_radix(args, 2).map_err(|_| AvcErr::BadBinary(String::from(args)))?;
                ret.push(Word::Byte(b))
            }
            "abspad" => {
                let pad = u16::from_str_radix(args, 16).map_err(|_| AvcErr::BadHex(String::from(args)))?;
                ret.push(Word::AbsPad(pad))
            }
            _ => return Err(AvcErr::UnrecognisedDirective(String::from(dir)))
        }
        Ok(ret)
    }
}

#[derive(Debug)]
enum Word<'a> {
    Byte(u8), // ops, literals, everything

    Lbl(&'a str),
    LblCall(&'a str, LblKind),

    AbsPad(u16),
    RelPad(u16),
    Align(u16),
}
impl Word<'_> {
    fn next_offset(&self, cur: usize) -> usize {
        match self {
            Word::RelPad(p) => cur + *p as usize,
            Word::AbsPad(p) => *p as usize,
            Word::Lbl(_) => cur,
            Word::LblCall(_, LblKind::Abs) => cur + 2,
            _ => cur + 1
        }
    }
}
#[derive(Debug)]
enum LblKind {
    Abs, Rel, Zpg
}
