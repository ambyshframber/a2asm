use crate::utils::*;

#[derive(Debug, PartialEq)]
pub struct AvcMacro {
    text: String,
    args: Vec<String>
}

impl AvcMacro {
    pub fn new(mac: &str) -> Result<AvcMacro> {
        let segments = split_bracket_groups(mac, ',');
        let args = segments[1];
        let args = &args[1..args.len() - 1];
        let args = args.split(',').map(|s| format!("${}", s.trim())).collect();
        let text = segments[2];
        let text = text[1..text.len() - 1].into();

        Ok(AvcMacro {
            text, args
        })
    }
    pub fn expand(&self, args: Vec<&str>) -> String {
        assert_eq!(args.len(), self.args.len()); // lazy

        let mut expansion = self.text.clone();
        for (arg, trig) in args.iter().zip(&self.args) {
            expansion = expansion.replace(trig, arg)
        }

        expansion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_create() {
        let m = AvcMacro::new("aaa, (aaa), (LIT #01 CLC ADC)");
        let m_correct = AvcMacro { text: "LIT #01 CLC ADC".into(), args: vec!["$aaa".into()] };
        assert_eq!(m, Ok(m_correct))
    }

    #[test]
    fn mac_expand_1() {
        let m = AvcMacro::new("aaa, (arg), (TEST $arg TEST)").unwrap();
        let exp = m.expand(vec!["beans"]);
        assert_eq!(exp, String::from("TEST beans TEST"))
    }
}
