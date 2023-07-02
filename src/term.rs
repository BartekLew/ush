use std::io::Read;
use std::io::Write;
use std::str;
use std::collections::HashMap;

use crate::hint::*;
use crate::autocomp::*;

type KAHandler = for <'a> fn(TermReader<'a>, &'a ShCommands, &[u8]) -> (bool, TermReader<'a>);

#[derive(Clone,Copy)]
pub enum KeyAction {
    Action(KAHandler),
}

impl KeyAction {
    fn run<'a> (&self, tr: TermReader<'a>, hints: &'a ShCommands, keys: &[u8]) -> (bool, TermReader<'a>) {
        match self {
            KeyAction::Action(x) => x(tr, hints, keys)
        }
    }
}

type KeyBind = HashMap<u8,KeyAction>;

pub struct TermReader<'a> {
    pub current : String,
    pub args : Vec<String>,
    pub chint : Option<ExcerptIter<'a, String>>,
    pub key_map : KeyBind,
    pub elsekey : Option<KeyAction>,
}

impl<'a> TermReader<'a> {
    pub fn new(keys : KeyBind, elsekey : Option<KeyAction>) -> Self {
        TermReader {
            current: String::from(""),
            args: vec![],
            chint: None,
            key_map: keys,
            elsekey: elsekey
        }
    }

    pub fn pushstr(self) -> TermReader<'a> {
        if self.current.len() > 0 {
            let mut na = self.args;
            na.push(self.current.clone());

            TermReader{ current: String::from(""),
                        args: na,
                        chint: self.chint,
                        elsekey: self.elsekey,
                        key_map: self.key_map}
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
        TermReader { current: val, args: self.args, chint: self.chint,
                     key_map: self.key_map, elsekey: self.elsekey}
    }

    pub fn accept<'b>(self, hints : &'b ShCommands, keys : &[u8]) -> (bool, TermReader<'b>)
            where 'a : 'b {
        if self.key_map.contains_key(&keys[0]) {
            let x = self.key_map[&keys[0]];
            x.run(self, hints, keys)
        } else {
            match self.elsekey {
                Some(x) => x.run(self, hints, keys),
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

pub fn reading(input : &mut dyn Read) -> Vec<String> {
    let mut buff : [u8;10] = [0;10];
    let hints = ShCommands::new();
    let mut tr = default_term();

    while match input.read(&mut buff) {
        Ok(len) => { let (cont, ntr) = tr.accept(&hints, &buff[0..len]);
                     tr = ntr;
                     cont},
        Err(e) => {
            println!("ERROR: {}", e);
            false
        }
    } {}

    tr.args
}
