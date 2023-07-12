use termios::*;
use std::os::unix::io::AsRawFd;
use std::process::{Command, Stdio, ChildStdin};

mod hint;
mod term;
mod autocomp;
use crate::term::*;

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

    let mut input = std::io::stdin();
    let ifd = input.as_raw_fd();

    let mut tos = Termios::from_fd(ifd).expect("This program can't be piped");
    tos.c_lflag &= !(ECHO | ICANON);
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();

    let ans = match output {
        Some(o) => reading(&mut input, o),
        None => reading(&mut input, std::io::stdout())
    };
    println!("{}", ans.join(","));

    tos.c_lflag |= ECHO | ICANON;
    tcsetattr(ifd, TCSAFLUSH, &tos).unwrap();
}
