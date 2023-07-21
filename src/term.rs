use std::io::Write;
use std::collections::HashMap;
use std::os::unix::prelude::AsRawFd;
use std::os::unix::prelude::RawFd;

use crate::autocomp::*;
use crate::hint::*;
use crate::fdmux::*;

pub struct Reading {
    pub tbc : bool,
    pub output : Option<Vec<u8>>
}

impl Reading {
    pub fn finished(output: Option<Vec<u8>>) -> Self {
        Reading { tbc: false, output: output }
    }

    pub fn tbc(output: Option<Vec<u8>>) -> Self {
        Reading { tbc: true, output: output }
    }
}

type KAHandler<T> = fn(&mut TermReader<T>, &[u8]) -> Reading;

pub enum KeyAction<T:DefaultVal> {
    Action(KAHandler<T>),
}

impl<T:DefaultVal> Copy for KeyAction<T> {}
impl<T:DefaultVal> Clone for KeyAction<T> {
    fn clone(&self) -> KeyAction<T> {
        *self
    }
}

impl<T:DefaultVal> KeyAction<T> {
    fn run (&self, tr: &mut TermReader<T>, keys: &[u8]) -> Reading {
        match self {
            KeyAction::Action(x) => x(tr, keys)
        }
    }
}

pub type KeyBind<T> = HashMap<u8,KeyAction<T>>;

pub trait DefaultVal {
    fn val(&self) -> &Vec<String>;
}

pub struct TermReader<T:DefaultVal> {
    pub ctx: T,
    pub key_map : KeyBind<T>,
    pub elsekey : KeyAction<T>,
}

impl<T:DefaultVal> TermReader<T> {
    pub fn new(ctx: T, keys : KeyBind<T>, elsekey : KeyAction<T>) -> Self {
        TermReader {
            ctx: ctx,
            key_map: keys,
            elsekey: elsekey
        }
    }

    pub fn set_mapping(&mut self, keys: KeyBind<T>, elsekey: KeyAction<T>) {
        self.key_map = keys;
        self.elsekey = elsekey;
    }

    pub fn accept(&mut self, keys : &[u8]) -> Reading {
        if self.key_map.contains_key(&keys[0]) {
            let x = self.key_map[&keys[0]];
            x.run(self, keys)
        } else {
            let f = self.elsekey;
            f.run(self, keys)
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

pub struct TermProc<'a,T:Muxable> {
    input: T,
    tr: TermReader<TermCtx<'a>>
}

impl <'a, T:Muxable> TermProc<'a,T> {
    pub fn new(input: T, hints: &'a ShCommands) -> Self {
        TermProc { input: input, tr: default_term(hints) }
    }
}

impl <'a, T:Muxable> AsRawFd for TermProc<'a,T> {
    fn as_raw_fd(&self) -> RawFd { self.input.as_raw_fd() }
}

impl <'a, T:Muxable> ReadStr for TermProc<'a,T> {
    fn read_str(&mut self) -> Result<Vec<u8>, StreamEvent> {
        match self.input.read_str() {
            Ok(s) => { let status = self.tr.accept(s.as_ref());
                       match status.tbc {
                            true => Ok(status.output.unwrap_or(vec![])),
                            false => Err(StreamEvent::Eof)
                       }
                     },
            Err(e) => Err(e)
        }
    }
}

impl <'a, T:Muxable> Muxable for TermProc<'a, T> {}

