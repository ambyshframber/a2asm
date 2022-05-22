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
                self.labels.insert(l, counter as u16);
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
            match word {
                Word::Byte(b) => set_vec_at(&mut ret, *b, counter),
                Word::LblCall(l, k) => {
                    let addr = self.labels.get(l).ok_or(AvcErr::UndefinedLabel(String::from(*l)))?;
                    match k {
                        LblKind::Abs => {
                            // increase by 300 to compensate for start point
                            // decrease by 1 to compensate for pc increasing after jump
                            let [hb, lb] = (addr + 0x02ff).to_be_bytes();
                            set_vec_at(&mut ret, hb, counter);
                            set_vec_at(&mut ret, lb, counter + 1);
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
        println!("parsing {}", s);
        match &s[..1] { // tokens should never be 0 length
            "." => {
                self.process_directive(&s[1..])
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
        if !args.ends_with(')') {
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
            "hex" => {
                let b = u8::from_str_radix(args, 16).map_err(|_| AvcErr::BadHex(String::from(args)))?;
                ret.push(Word::Byte(b))
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
