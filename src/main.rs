use termios::*;
use std::os::unix::io::AsRawFd;
use std::process::{Command, Stdio, ChildStdin};

mod hint;
mod term;
mod autocomp;
mod fdmux;
use crate::term::*;
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

    let inpipe = NamedReadPipe::new("/tmp/ush".to_string()).unwrap();
    let mut inmux = FdMux::new(2)
                          .add(&input, Action::Read)
                          .add(&inpipe, Action::Read);

    let mut tos = Termios::from_fd(ifd).expect("This program can't be piped");
    tos.c_lflag &= !(ECHO | ICANON);
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();

    let ans = match output {
        Some(o) => reading(&mut inmux, o),
        None => reading(&mut inmux, std::io::stdout())
    };
    println!("{}", ans.join(","));

    tos.c_lflag |= ECHO | ICANON;
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();
}
