use termios::*;
use std::os::unix::io::AsRawFd;
use std::io::Read;
use std::io::Write;
use std::str;
use std::collections::HashMap;
mod hint;

use crate::hint::*;

type KAHandler = for <'a> fn(TermReader<'a>, &'a TermCfg, &[u8]) -> (bool, TermReader<'a>);

#[derive(Clone,Copy)]
enum KeyAction {
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

struct TermCfg {
    key_map : KeyBind,
    elsekey : Option<KeyAction>,
    hints : ShCommands,
}

impl TermCfg {
    fn new(keys : KeyBind, elsekey : Option<KeyAction>) -> Self {
        TermCfg {
            key_map: keys,
            elsekey: elsekey,
            hints: ShCommands::new(),
        }
    }
}

struct TermReader<'a> {
    current : String,
    pub args : Vec<String>,
    chint : Option<ExcerptIter<'a, String>>
}

impl<'a> TermReader<'a> {
    fn new() -> Self {
        TermReader {
            current: String::from(""),
            args: vec![],
            chint: None
        }
    }

    fn pushstr(self) -> TermReader<'a> {
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

    fn autocomplete(mut self) -> TermReader<'a> {
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

fn echo(keys: &[u8]) {
    std::io::stdout().write(keys).unwrap();           
    std::io::stdout().flush().unwrap();
}

struct Term;
impl Term {
    fn echo(&self, keys: &[u8]) -> &Term {
        std::io::stdout().write(keys).unwrap();           
        self
    }

    fn endline(&self) -> &Term {
        std::io::stdout().write(b"\x1b[K").unwrap();
        self
    }

    fn hmove(&self, amount : i32) -> &Term {
        if amount > 0 {
            std::io::stdout().write(format!("\x1b[{}C", amount).as_bytes()).unwrap();
        } else if amount < 0 {
            std::io::stdout().write(format!("\x1b[{}D", -amount).as_bytes()).unwrap();
        }

        self
    }

    fn move_left(&self, amount : usize) -> &Term {
        self.hmove(-(amount as i32))
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        std::io::stdout().flush().unwrap();
    }
}

fn reading(input : &mut dyn Read, cfg : TermCfg) -> Vec<String> {
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

fn ac_elsekey<'a> (tr: TermReader<'a>, cfg: &'a TermCfg, keys: &[u8]) -> (bool, TermReader<'a>) {
    if tr.args.len() > 0 {
        echo(keys);
        let nc = tr.current.clone() + str::from_utf8(keys).unwrap();
        return (true, tr.with_current(nc))
    }

    let trial = tr.current.clone() + str::from_utf8(keys).unwrap();
    match cfg.hints.for_prefix(&trial) {
        Some(mut it) => {
            let first = it.get().unwrap();
            match first.get(tr.current.len()..) {
                Some(s) => {
                    Term.echo(s.as_bytes())
                        .endline()
                        .move_left(s.len() - 1);
                }, None => ()
            }
    
            (true, TermReader {
                current: trial,
                chint: Some(it),
                args: tr.args
            })
        },
        None => (true, tr)
    }
}

fn ac_bs<'a> (tr: TermReader<'a>, cfg: &'a TermCfg, keys: &[u8]) -> (bool, TermReader<'a>) {
    if tr.args.len() > 0 {
        echo(keys);
        let mut nc = tr.current.clone();
        nc.pop();
        return (true, tr.with_current(nc))
    }

    if tr.current.len() == 0 {
        return (true, tr);
    }

    let mut trial = tr.current.clone();
    trial.pop();
    match cfg.hints.for_prefix(&trial) {
        Some(mut it) => {
            let first = it.get().unwrap();
            let term = Term;
            term.hmove(-1);
            match first.get(trial.len()..) {
                Some(s) => { term.echo(s.as_bytes())
                                 .endline()
                                 .hmove(-(s.len() as i32)); }
                None => ()
            }
    
            (true, TermReader {
                current: trial,
                chint: Some(it),
                args: tr.args
            })
        },
        None => (true, tr)
    }
}

fn ac_space<'a> (tr: TermReader<'a>, _cfg: &'a TermCfg,  _: &[u8]) -> (bool,TermReader<'a>) {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            return (true, tr.pushstr());
        }
        return (true, tr)
    }

    (true, tr.autocomplete())
}

fn ac_ret<'a> (tr: TermReader<'a>, _cfg: &'a TermCfg, _: &[u8]) -> (bool, TermReader<'a>) {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            return (false, tr.pushstr());
        }
        return (false, tr)
    }

    let ntr = tr.autocomplete();
    Term.echo(b"\n");

    (false,ntr)
}

fn main() {
    let mut input = std::io::stdin();
    let ifd = input.as_raw_fd();

    let initial_keys = HashMap::from([
        (b' ', KeyAction::Action(ac_space)), 
        (b'\n', KeyAction::Action(ac_ret)),
        (0x7f, KeyAction::Action(ac_bs)),
    ]);

    let mut tos = Termios::from_fd(ifd).unwrap();
    tos.c_lflag &= !(ECHO | ICANON);
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();

    let cfg = TermCfg::new(initial_keys, Some(KeyAction::Action(ac_elsekey)));
    let args = reading(&mut input, cfg);
    println!("{}", args.join(","));

    tos.c_lflag |= ECHO | ICANON;
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();
}
