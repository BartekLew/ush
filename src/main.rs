use termios::*;
use std::os::unix::io::AsRawFd;

mod hint;
mod term;
mod autocomp;
use crate::term::*;

fn main() {
    let mut input = std::io::stdin();
    let ifd = input.as_raw_fd();

    let mut tos = Termios::from_fd(ifd).unwrap();
    tos.c_lflag &= !(ECHO | ICANON);
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();

    let args = reading(&mut input);
    println!("{}", args.join(","));

    tos.c_lflag |= ECHO | ICANON;
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();
}
