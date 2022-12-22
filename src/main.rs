use termios::*;
use std::os::unix::io::AsRawFd;
use std::io::Read;
use std::io::Error;
use std::io::ErrorKind;
use std::str;

fn reading(input : &mut dyn Read, act : fn(&str)) {
    let mut buff : [u8;10] = [0;10];

    while match input.read(&mut buff)
                     .and_then(|_| str::from_utf8(&buff)
                                      .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))) { 
        Ok(n) => { act(n); true },
        Err(e) => {
            println!("ERROR: {}", e);
            false
        }
    } {}
}

fn main() {
    let mut input = std::io::stdin();
    let ifd = input.as_raw_fd();

    let mut tos = Termios::from_fd(ifd).unwrap();
    tos.c_lflag &= !(ECHO | ICANON);
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();

    reading(&mut input, |str| println!(">> {}", str));

    tos.c_lflag |= ECHO | ICANON;
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();
}
