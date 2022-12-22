use termios::*;
use std::os::unix::io::AsRawFd;
use std::io::Read;
use std::io::Write;
use std::str;
use std::collections::HashMap;

#[derive(Clone,Copy)]
struct KeyAction {
    item_end : bool,
    record_end : bool
}

type KeyBind = HashMap<u8,KeyAction>;

struct TermReader {
    key_map : KeyBind,
    current : String,
    pub args : Vec<String>
}

impl TermReader {
    fn new(keys : KeyBind) -> Self {
        TermReader {
            key_map: keys,
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

    fn accept(&mut self, keys : &[u8]) -> bool {
        std::io::stdout().write(keys).unwrap();           
        std::io::stdout().flush().unwrap();

        if self.key_map.contains_key(&keys[0]) {
            let flags = self.key_map[&keys[0]];
            if flags.item_end { self.pushstr(); }
            !flags.record_end
        } else {
            self.current = self.current.clone() + str::from_utf8(keys).unwrap();
            true
        }
    }
}

fn reading(input : &mut dyn Read, tr : &mut TermReader) {
    let mut buff : [u8;10] = [0;10];

    while match input.read(&mut buff) {
        Ok(_) => { tr.accept(&buff) },
        Err(e) => {
            println!("ERROR: {}", e);
            false
        }
    } {}
}

fn main() {
    let mut input = std::io::stdin();
    let ifd = input.as_raw_fd();

    let initial_keys = HashMap::from([
        (b' ', KeyAction { item_end: true, record_end: false }),
        (b'\n', KeyAction { item_end: true, record_end: true })
    ]);

    let mut tos = Termios::from_fd(ifd).unwrap();
    tos.c_lflag &= !(ECHO | ICANON);
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();

    let mut tr = TermReader::new(initial_keys);
    reading(&mut input, &mut tr);
    println!("{}", tr.args.join(","));

    tos.c_lflag |= ECHO | ICANON;
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();
}
