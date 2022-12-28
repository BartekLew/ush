use termios::*;
use std::os::unix::io::AsRawFd;
use std::io::Read;
use std::io::Write;
use std::str;
use std::collections::HashMap;
mod hint;

use crate::hint::ShCommands;

type KAHandler = fn(&mut TermReader, &[u8]) -> bool;

#[derive(Clone,Copy)]
enum KeyAction {
    Action(KAHandler),
}

impl KeyAction {
    fn run (&self, tr: &mut TermReader, keys: &[u8]) -> bool {
        match self {
            KeyAction::Action(x) => x(tr, keys)
        }
    }
}

type KeyBind = HashMap<u8,KeyAction>;

struct TermReader {
    key_map : KeyBind,
    elsekey : Option<KeyAction>,
    current : String,
    hints : Option<ShCommands>,
    pub args : Vec<String>
}

impl TermReader {
    fn new(keys : KeyBind, elsekey : Option<KeyAction>) -> Self {
        TermReader {
            key_map: keys,
            elsekey: elsekey,
            current: String::from(""),
            hints: None,
            args: vec![]
        }
    }

    fn pushstr(&mut self) {
        if self.current.len() > 0 {
            self.args.push(self.current.clone());
            self.current = String::from("");
        }
    }

    fn autocomplete(&mut self) {
        if self.current.len() > 0 {
            match &self.hints {
                Some(hs) => {
                    let chint = hs.for_prefix(&self.current)[0];
                    Term.hmove((chint.len() - self.current.len() + 1) as i32);
                    self.current = chint.to_string();
                    self.pushstr();
                },
                None => {}
            }
        }
    }

    fn accept(&mut self, keys : &[u8]) -> bool {
        if self.key_map.contains_key(&keys[0]) {
            let x = self.key_map[&keys[0]];
            x.run(self, keys)
        } else {
            match self.elsekey {
                Some(x) => x.run(self,keys),
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

fn reading(input : &mut dyn Read, tr : &mut TermReader) {
    let mut buff : [u8;10] = [0;10];

    while match input.read(&mut buff) {
        Ok(len) => { tr.accept(&buff[0..len]) },
        Err(e) => {
            println!("ERROR: {}", e);
            false
        }
    } {}
}

fn ac_elsekey(tr: &mut TermReader, keys: &[u8]) -> bool {
    if tr.args.len() > 0 {
        echo(keys);
        tr.current = tr.current.clone() + str::from_utf8(keys).unwrap();
        return true
    }

    if tr.hints.is_none() {
        tr.hints = Some(ShCommands::new());
    }

    let trial = tr.current.clone() + str::from_utf8(keys).unwrap();
    match &tr.hints {
        Some(nh) => { 
            let newhints = nh.for_prefix(&trial);
            if newhints.len() > 0 {
                let first = newhints[0];
                Term.echo(first.get(tr.current.len()..).unwrap().as_bytes())
                    .endline()
                    .hmove(-((first.len() - tr.current.len() - 1) as i32));
                
                tr.current = tr.current.clone() + str::from_utf8(keys).unwrap();
            }
        }
        None => {}
    }

    true
}

fn ac_space (tr: &mut TermReader, _: &[u8]) -> bool {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            tr.pushstr();
        }
        return true
    }

    tr.autocomplete();

    true
}

fn ac_ret (tr: &mut TermReader, _: &[u8]) -> bool {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            tr.pushstr();
        }
        return false
    }

    tr.autocomplete();
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

    let mut tr = TermReader::new(initial_keys, Some(KeyAction::Action(ac_elsekey)));
    reading(&mut input, &mut tr);
    println!("{}", tr.args.join(","));

    tos.c_lflag |= ECHO | ICANON;
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();
}
