use std::io::Read;
use std::io::Write;
use std::collections::HashMap;

use crate::hint::*;
use crate::autocomp::*;

pub struct Reading<'a> {
    tbc : bool,
    mode : TermReader<'a>,
    output : Option<String>
}

impl <'a> Reading<'a> {
    pub fn finished(mode: TermReader<'a>, output: Option<String>) -> Self {
        Reading { mode: mode, tbc: false, output: output }
    }

    pub fn tbc(mode: TermReader<'a>, output: Option<String>) -> Self {
        Reading { mode: mode, tbc: true, output: output }
    }

    pub fn commit<W: Write>(&self, out: &mut W) -> bool {
        match &self.output {
            Some(x) => {
                out.write(x.as_bytes()).unwrap();
                out.flush().unwrap();
            },
            None => {}
        }

        self.tbc
    }
}

type KAHandler = for <'a> fn(TermReader<'a>, &'a ShCommands, &[u8]) -> Reading<'a>;

#[derive(Clone,Copy)]
pub enum KeyAction {
    Action(KAHandler),
}

impl KeyAction {
    fn run<'a> (&self, tr: TermReader<'a>, hints: &'a ShCommands, keys: &[u8]) -> Reading<'a> {
        match self {
            KeyAction::Action(x) => x(tr, hints, keys)
        }
    }
}

pub type KeyBind = HashMap<u8,KeyAction>;

pub struct TermReader<'a> {
    pub output : String,
    pub current : String,
    pub args : Vec<String>,
    pub chint : Option<ExcerptIter<'a, String>>,
    pub key_map : KeyBind,
    pub elsekey : KeyAction,
}

impl<'a> TermReader<'a> {
    pub fn new(keys : KeyBind, elsekey : KeyAction) -> Self {
        TermReader {
            output: String::from(""),
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

            TermReader{ output: self.output,
                        current: String::from(""),
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
        TermReader { output: self.output, current: val, args: self.args, chint: self.chint,
                     key_map: self.key_map, elsekey: self.elsekey}
    }

    pub fn accept<'b>(self, hints : &'b ShCommands, keys : &[u8]) -> Reading<'b>
            where 'a : 'b {
        if self.key_map.contains_key(&keys[0]) {
            let x = self.key_map[&keys[0]];
            x.run(self, hints, keys)
        } else {
            let f = self.elsekey;
            f.run(self, hints, keys)
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

pub fn reading<W: Write>(input : &mut dyn Read, mut output: W) -> Vec<String> {
    let mut buff : [u8;10] = [0;10];
    let hints = ShCommands::new();
    let mut tr = default_term();

    while match input.read(&mut buff) {
        Ok(len) => { let status = tr.accept(&hints, &buff[0..len]);
                     status.commit(&mut output);
                     tr = status.mode;
                     status.tbc},
        Err(e) => {
            println!("ERROR: {}", e);
            false
        }
    } {}

    tr.args
}
