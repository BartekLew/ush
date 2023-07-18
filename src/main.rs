use termios::*;
use std::os::unix::io::AsRawFd;
use std::process::{Command, Stdio, ChildStdin};

mod hint;
mod term;
mod autocomp;
mod fdmux;
use crate::term::*;
use crate::hint::*;
use crate::fdmux::*;

fn main() {
    let mut cmdline = std::env::args().skip(1);
    let output = cmdline.next().map(|runcmd| -> ChildStdin {
            let args : Vec<String> = cmdline.collect();
            Command::new(runcmd)
                         .args(args)
                         .stdin(Stdio::piped())
                         .stdout(Stdio::inherit())
                         .spawn()
                         .expect("can't run command")
                         .stdin.unwrap()
        });

    let input = std::io::stdin();
    let ifd = input.as_raw_fd();

    let mut tos = Termios::from_fd(ifd).expect("This program can't be piped");
    tos.c_lflag &= !(ECHO | ICANON);
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();

    let mut inpipe = NamedReadPipe::new("/tmp/ush".to_string()).unwrap();

    let hints = ShCommands::new();
    let mut cmdpipe = TermProc::new(input, &hints);
    let inmux = FdMux::new(2)
                      .add(&mut cmdpipe)
                      .add(&mut inpipe);
                      
    match output {
        Some(o) => inmux.pass_to(o),
        None => inmux.pass_to(std::io::stdout())
    };

    tos.c_lflag |= ECHO | ICANON;
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();
}
