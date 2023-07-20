mod hint;
mod term;
mod autocomp;
mod fdmux;
use crate::term::*;
use crate::hint::*;
use crate::fdmux::*;

use std::io::Write;

fn main() {
    let mut cmdline = std::env::args().skip(1);
    let mut output = cmdline.next().map(|runcmd| -> IOPipe {
            let args : Vec<String> = cmdline.collect();
            Pty::new().unwrap()
                      .spawn_output(runcmd, args).unwrap()
        });


    let mut inpipe = EchoPipe{
                        input: NamedReadPipe::new("/tmp/ush".to_string())
                                             .unwrap() };

    let hints = ShCommands::new();
    let mut cmdpipe = TermProc::new(StdinReadKey::new(), &hints);

    let mut stdout = std::io::stdout();
    let out: &mut dyn Write = output.as_mut()
                                    .map(|x| -> &mut dyn Write { x })
                                    .unwrap_or(&mut stdout );

    Topology::new(2)
             .add(Destination::new(out, vec![&mut cmdpipe, &mut inpipe]))
             .run();
}
