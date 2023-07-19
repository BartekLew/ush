use std::process::{Command, Stdio, Child};

mod hint;
mod term;
mod autocomp;
mod fdmux;
use crate::term::*;
use crate::hint::*;
use crate::fdmux::*;

pub trait PipeConsumer {
    fn consume(self, input: FdMux);
}

impl PipeConsumer for Child {
    fn consume(mut self, mut input: FdMux) {
        match &self.stdin {
            Some(output) => {
                input.pass_to(output);
                self.kill().unwrap();
            },
            None => {}
        }
    }
}

fn main() {
    let mut cmdline = std::env::args().skip(1);
    let output = cmdline.next().map(|runcmd| -> Child {
            let args : Vec<String> = cmdline.collect();
            Command::new(runcmd)
                         .args(args)
                         .stdin(Stdio::piped())
                         .stdout(Stdio::inherit())
                         .spawn()
                         .expect("can't run command")
        });

    let mut inpipe = EchoPipe{
                        input: NamedReadPipe::new("/tmp/ush".to_string())
                                             .unwrap() };

    let hints = ShCommands::new();
    let mut cmdpipe = TermProc::new(StdinReadKey::new(), &hints);
    let mut inmux = FdMux::new(2)
                      .add(&mut cmdpipe)
                      .add(&mut inpipe);
                      
    match output {
        Some(o) => o.consume(inmux),
        None => inmux.pass_to(std::io::stdout())
    };
}
