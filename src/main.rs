use termios::*;
use std::os::unix::io::AsRawFd;
use std::io::Read;
use std::io::Write;
use std::str;
use std::collections::HashMap;
mod hint;

use crate::hint::ShCommands;

type KAHandler = fn(&mut TermReader, &TermCfg, &[u8]) -> bool;

#[derive(Clone,Copy)]
enum KeyAction {
    Action(KAHandler),
}

impl KeyAction {
    fn run (&self, tr: &mut TermReader, cfg: &TermCfg, keys: &[u8]) -> bool {
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

struct TermReader {
    current : String,
    pub args : Vec<String>
}

impl TermReader {
    fn new() -> Self {
        TermReader {
            current: String::from(""),
            args: vec![]
        }
    }

    fn pushstr(&mut self) {
        if self.current.len() > 0 {
            self.args.push(self.current.clone());
            self.current = String::from("");
        }
    }

    fn autocomplete(&mut self, cfg : &TermCfg) {
        if self.current.len() > 0 {
            let chint = cfg.hints.for_prefix(&self.current)[0];
            Term.hmove((chint.len() - self.current.len() + 1) as i32);
            self.current = chint.to_string();
            self.pushstr();
        }
    }

    pub fn accept(&mut self, cfg : &TermCfg, keys : &[u8]) -> bool {
        if cfg.key_map.contains_key(&keys[0]) {
            let x = cfg.key_map[&keys[0]];
            x.run(self, cfg, keys)
        } else {
            match cfg.elsekey {
                Some(x) => x.run(self, cfg, keys),
                None => {
                    echo(keys);
                    self.current = self.current.clone() + str::from_utf8(keys).unwrap();
                    true
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
}

impl Drop for Term {
    fn drop(&mut self) {
        std::io::stdout().flush().unwrap();
    }
}

fn reading(input : &mut dyn Read, tr : &mut TermReader, cfg : TermCfg) {
    let mut buff : [u8;10] = [0;10];

    while match input.read(&mut buff) {
        Ok(len) => { tr.accept(&cfg, &buff[0..len]) },
        Err(e) => {
            println!("ERROR: {}", e);
            false
        }
    } {}
}

fn ac_elsekey(tr: &mut TermReader, cfg: &TermCfg, keys: &[u8]) -> bool {
    if tr.args.len() > 0 {
        echo(keys);
        tr.current = tr.current.clone() + str::from_utf8(keys).unwrap();
        return true
    }

    let trial = tr.current.clone() + str::from_utf8(keys).unwrap();
    let newhints = cfg.hints.for_prefix(&trial);
    if newhints.len() > 0 {
        let first = newhints[0];
        Term.echo(first.get(tr.current.len()..).unwrap().as_bytes())
            .endline()
            .hmove(-((first.len() - tr.current.len() - 1) as i32));
        
        tr.current = tr.current.clone() + str::from_utf8(keys).unwrap();
    }

    true
}

fn ac_space (tr: &mut TermReader, cfg: &TermCfg,  _: &[u8]) -> bool {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            tr.pushstr();
        }
        return true
    }

    tr.autocomplete(cfg);

    true
}

fn ac_ret (tr: &mut TermReader, cfg: &TermCfg, _: &[u8]) -> bool {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            tr.pushstr();
        }
        return false
    }

    tr.autocomplete(cfg);
    Term.echo(b"\n");

    false
}

fn main() {
    let mut input = std::io::stdin();
    let ifd = input.as_raw_fd();

    let initial_keys = HashMap::from([
        (b' ', KeyAction::Action(ac_space)), 
        (b'\n', KeyAction::Action(ac_ret))
    ]);

    let mut tos = Termios::from_fd(ifd).unwrap();
    tos.c_lflag &= !(ECHO | ICANON);
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();

    let cfg = TermCfg::new(initial_keys, Some(KeyAction::Action(ac_elsekey)));
    let mut tr = TermReader::new();
    reading(&mut input, &mut tr, cfg);
    println!("{}", tr.args.join(","));

    tos.c_lflag |= ECHO | ICANON;
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();
}
