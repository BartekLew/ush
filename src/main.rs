mod hint;
mod term;
mod autocomp;
mod fdmux;
use crate::term::*;
use crate::hint::*;
use crate::fdmux::*;

fn main() {
    let mut cmdline = std::env::args().skip(1);
    let output = cmdline.next().map(|runcmd| -> IOPipe {
            let args : Vec<String> = cmdline.collect();
            Pty::new().unwrap()
                      .spawn_output(runcmd, args).unwrap()
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
