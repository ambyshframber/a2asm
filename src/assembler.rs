use crate::utils::*;
use std::collections::HashMap;
use crate::avcmacro::AvcMacro;

pub struct Assembler<'a> {
    code: &'a str,
    words: Vec<Word>,
    labels: HashMap<String, u16>,
    macros: HashMap<String, AvcMacro>
}

impl<'a> Assembler<'a> {
    pub fn new(code: &'a str) -> Assembler {
        Assembler {
            code,
            words: Vec::new(),
            labels: HashMap::new(),
            macros: HashMap::new()
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
            let token = token?;
            //println!("{}", token);
            let words = match self.token_to_words(token.token) {
                Ok(v) => v,
                Err(e) => {
                    println!("error on line {}: {}", token.line, e);
                    return Err(e)
                }
            };
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
                self.labels.insert(l.clone(), counter as u16);
            }
            if word.is_byte_like() && counter < 0x0300 { // in zpg/stack
                println!("{:?}", word);
                return Err(AvcErr::OpNotInCodeSpace)
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
                    let addr = self.labels.get(l).ok_or_else(|| AvcErr::UndefinedLabel(l.clone()))?;
                    match k {
                        LblKind::Abs => {
                            let [hb, lb] = (addr).to_be_bytes();
                            set_vec_at(&mut ret, hb, counter_inner);
                            set_vec_at(&mut ret, lb, counter_inner + 1);
                        }
                        LblKind::Rel => {
                            // signed 8 bit
                            // if counter > addr, jump forwards ie. ctr - addr
                            // if addr > counter, jump back ie. (addr - ctr) * -1
                            // sub 1 to account for Things

                            let rel: i8 = (((*addr as isize) - (counter as isize)) - 1)
                                .try_into().map_err(|_| AvcErr::RelJumpTooLarge)?;
                            set_vec_at(&mut ret, rel as u8, counter_inner)
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

    fn token_to_words(&mut self, s: &str) -> Result<Vec<Word>> { // returns a vec because macros and strings
        //println!("parsing {}", s);
        match &s[..1] { // tokens should never be 0 length
            "." => {
                self.process_directive(s[1..].into())
            }
            "#" => { // raw hex
                let b = u8::from_str_radix(&s[1..], 16).map_err(|_| AvcErr::BadInt(String::from(&s[1..])))?;
                Ok(vec![Word::Byte(b)])
            }
            "\"" => {
                Ok(s[1..].bytes().map(Word::Byte).collect()) // L E V E R A G E
            }
            "'" => {
                let c = s[1..].chars().next().unwrap();
                if c.is_ascii() {
                    Ok(vec![Word::Byte(c as u8)])
                }
                else {
                    Err(AvcErr::MultibyteChar(c))
                }
            }
            "@" => {
                Ok(vec![Word::LblCall(s[1..].into(), LblKind::Abs)])
            }
            "^" => {
                Ok(vec![Word::LblCall(s[1..].into(), LblKind::Rel)])
            }
            "%" => {
                match s[1..].split_once('(') {
                    Some((name, args)) => {
                        self.expand_macro(name, &args[..args.len() - 1])
                    }
                    None => {
                        self.expand_macro(&s[1..], "")
                    }
                }
            }
            _ => {
                let op = op_to_byte(s)?;
                Ok(vec![Word::Byte(op)])
            }
        }
    }
    // shave off . before calling
    fn process_directive(&mut self, dir: &str) -> Result<Vec<Word>> {
        let (directive_name, args) = dir.split_once('(').ok_or_else(|| AvcErr::MalformedDirective(String::from(dir)))?;
        if !args.ends_with(')') { // not actually sure this can happen but it doesn't hurt to check
            return Err(AvcErr::MalformedDirective(String::from(dir)))
        }
        let args = &args[..args.len() - 1];
        let mut ret = Vec::new();
        match directive_name {
            "label" | "lbl" => {
                ret.push(Word::Lbl(args.into()))
            }
            "absc" | "abscall" => {
                ret.push(Word::LblCall(args.into(), LblKind::Abs))
            }
            "relcall" => {
                ret.push(Word::LblCall(args.into(), LblKind::Rel))
            }
            "hex" | "x" => {
                let b = u8::from_str_radix(args, 16).map_err(|_| AvcErr::BadInt(String::from(args)))?;
                ret.push(Word::Byte(b))
            }
            "x2" => {
                let [hb, lb] = u16::from_str_radix(args, 16)
                    .map_err(|_| AvcErr::BadInt(String::from(args)))?
                    .to_be_bytes();
                ret.push(Word::Byte(hb));
                ret.push(Word::Byte(lb));
            }
            "b" => {
                let b = u8::from_str_radix(args, 2).map_err(|_| AvcErr::BadInt(String::from(args)))?;
                ret.push(Word::Byte(b))
            }
            "d" => {
                let b = args.parse::<u8>().map_err(|_| AvcErr::BadInt(String::from(args)))?;
                ret.push(Word::Byte(b))
            }
            "s" => {
                ret.extend(args.bytes().map(Word::Byte)) // LEVERAGE
            }
            "abspad" => {
                let pad = u16::from_str_radix(args, 16).map_err(|_| AvcErr::BadInt(String::from(args)))?;
                ret.push(Word::AbsPad(pad))
            }
            "relpad" => {
                let pad = u16::from_str_radix(args, 16).map_err(|_| AvcErr::BadInt(String::from(args)))?;
                ret.push(Word::RelPad(pad))
            }
            "align" => {
                let amt = u16::from_str_radix(args, 16).map_err(|_| AvcErr::BadInt(String::from(args)))?;
                ret.push(Word::Align(amt))
            }
            "defmac" => {
                let m = AvcMacro::new(args)?;
                let m_name = args.split_once(',').unwrap().0;
                self.macros.insert(m_name.into(), m);
            }
            _ => return Err(AvcErr::UnrecognisedDirective(String::from(dir)))
        }
        Ok(ret)
    }

    fn expand_macro(&mut self, name: &str, args: &str) -> Result<Vec<Word>> {
        let m = self.macros.get(name).ok_or_else(|| AvcErr::UndefinedMacro(name.into()))?;
        let args_s = args.split(',').map(|a| a.trim()).collect();
        let m_exp = m.expand(args_s);

        self.process_expanded_macro(&m_exp)
    }

    fn process_expanded_macro(&mut self, mac: &str) -> Result<Vec<Word>> {
        let mut ret= Vec::new();
        let t = Tokeniser::new(mac);
        for token in t {
            let token = token?;
            ret.append(&mut self.token_to_words(token.token)?)
        }
        Ok(ret)
    }
}

#[derive(Debug)]
enum Word {
    Byte(u8), // ops, literals, everything

    Lbl(String),
    LblCall(String, LblKind),

    AbsPad(u16),
    RelPad(u16),
    Align(u16),
}
impl Word {
    fn next_offset(&self, cur: usize) -> usize {
        match self {
            Word::RelPad(p) => cur + *p as usize,
            Word::AbsPad(p) => *p as usize,
            Word::Lbl(_) => cur,
            Word::LblCall(_, LblKind::Abs) => cur + 2,
            Word::Align(amt) => {
                align(cur, *amt)
            }
            _ => cur + 1
        }
    }
    fn is_byte_like(&self) -> bool {
        !(
            matches!(self, Word::Lbl(_)) ||
            matches!(self, Word::AbsPad(_)) ||
            matches!(self, Word::RelPad(_))
        )
    }
}
#[allow(dead_code)]
#[derive(Debug)]
enum LblKind {
    Abs, Rel, Zpg
}
// round cur up to the next multiple of amt
fn align(cur: usize, amt: u16) -> usize {
    let amtu = amt as usize;
    if cur % amtu == 0 {
        cur
    }
    else {
        let muls = cur / amtu;
        (muls + 1) * amtu
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn align_test() {
        assert_eq!(align(1, 16), 16);
        assert_eq!(align(16, 16), 16);
        assert_eq!(align(28, 16), 32);
        assert_eq!(align(48, 16), 48);
        assert_eq!(align(28, 5), 30);
    }
}
