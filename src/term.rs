use std::io::Read;
use std::io::Write;
use std::str;
use std::collections::HashMap;

use crate::hint::*;

type KAHandler = for <'a> fn(TermReader<'a>, &'a TermCfg, &[u8]) -> (bool, TermReader<'a>);

#[derive(Clone,Copy)]
pub enum KeyAction {
    Action(KAHandler),
}

impl KeyAction {
    fn run<'a> (&self, tr: TermReader<'a>, cfg: &'a TermCfg, keys: &[u8]) -> (bool, TermReader<'a>) {
        match self {
            KeyAction::Action(x) => x(tr, cfg, keys)
        }
    }
}

type KeyBind = HashMap<u8,KeyAction>;

pub struct TermCfg {
    key_map : KeyBind,
    elsekey : Option<KeyAction>,
    pub hints : ShCommands,
}

impl TermCfg {
    pub fn new(keys : KeyBind, elsekey : Option<KeyAction>) -> Self {
        TermCfg {
            key_map: keys,
            elsekey: elsekey,
            hints: ShCommands::new(),
        }
    }
}

pub struct TermReader<'a> {
    pub current : String,
    pub args : Vec<String>,
    pub chint : Option<ExcerptIter<'a, String>>
}

impl<'a> TermReader<'a> {
    fn new() -> Self {
        TermReader {
            current: String::from(""),
            args: vec![],
            chint: None
        }
    }

    pub fn pushstr(self) -> TermReader<'a> {
        if self.current.len() > 0 {
            let mut na = self.args;
            na.push(self.current.clone());

            TermReader{ current: String::from(""),
                        args: na,
                        chint: self.chint }
        } else {
            self
        }
    }

    pub fn autocomplete(mut self) -> TermReader<'a> {
        if self.current.len() > 0 {
            match &self.chint {
                Some(ch) => {
                    match ch.peek() {
                        Some(chint) => {
                            Term.hmove((chint.len() - self.current.len() + 1) as i32);
                            self.current = chint.to_string();
                            self.pushstr()
                        },
                        None => self
                    }
                },
                None => self
            }
        } else {
            self
        }
    }

    pub fn with_current(self, val : String) -> TermReader<'a> {
        TermReader { current: val, args: self.args, chint: self.chint }
    }

    pub fn accept<'b>(self, cfg : &'b TermCfg, keys : &[u8]) -> (bool, TermReader<'b>)
            where 'a : 'b {
        if cfg.key_map.contains_key(&keys[0]) {
            let x = cfg.key_map[&keys[0]];
            x.run(self, cfg, keys)
        } else {
            match cfg.elsekey {
                Some(x) => x.run(self, cfg, keys),
                None => {
                    echo(keys);
                    let nc = self.current.clone() + str::from_utf8(keys).unwrap();
                    (true, self.with_current(nc))
                }
            }
        }
    }
}

pub fn echo(keys: &[u8]) {
    std::io::stdout().write(keys).unwrap();           
    std::io::stdout().flush().unwrap();
}

pub struct Term;
impl Term {
    pub fn echo(&self, keys: &[u8]) -> &Term {
        std::io::stdout().write(keys).unwrap();           
        self
    }

    pub fn endline(&self) -> &Term {
        std::io::stdout().write(b"\x1b[K").unwrap();
        self
    }

    pub fn hmove(&self, amount : i32) -> &Term {
        if amount > 0 {
            std::io::stdout().write(format!("\x1b[{}C", amount).as_bytes()).unwrap();
        } else if amount < 0 {
            std::io::stdout().write(format!("\x1b[{}D", -amount).as_bytes()).unwrap();
        }

        self
    }

    pub fn move_left(&self, amount : usize) -> &Term {
        self.hmove(-(amount as i32))
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        std::io::stdout().flush().unwrap();
    }
}

pub fn reading(input : &mut dyn Read, cfg : TermCfg) -> Vec<String> {
    let mut buff : [u8;10] = [0;10];
    let mut tr = TermReader::new();

    while match input.read(&mut buff) {
        Ok(len) => { let (cont, ntr) = tr.accept(&cfg, &buff[0..len]);
                     tr = ntr;
                     cont},
        Err(e) => {
            println!("ERROR: {}", e);
            false
        }
    } {}

    tr.args
}
