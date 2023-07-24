mod hint;
mod term;
mod autocomp;
mod fdmux;
use crate::term::*;
use crate::hint::*;
use crate::fdmux::*;

struct Sink {
    std: Option<std::io::Stdout>,
    pipe: Option<IOPipe>
}

impl Sink {
    fn new(pipe: Option<IOPipe>) -> Self {
        match pipe {
            Some(handle) => Sink { std: None, pipe: Some(handle) },
            None => Sink { std: Some(std::io::stdout()), pipe: None }
        }
    }
    
    fn run<T: Muxable, U: Muxable>(mut self, mut cmdpipe: T, mut inpipe: U)  {
        match self.pipe {
            Some(mut pipe) => {
                let mut second = pipe.clone();
                Topology::new(2)
                         .add(Destination::new(&mut std::io::stdout(), vec![&mut pipe]))
                         .add(Destination::new(&mut second, vec![&mut cmdpipe, &mut inpipe]))
                         .run();
            }, 
            None => {
                Topology::new(2)
                         .add(Destination::new(self.std.as_mut().unwrap(),
                              vec![&mut cmdpipe, &mut inpipe]))
                         .run();
            }
        }
    }
}

fn main() {
    let cmdline: Vec<String> = std::env::args().skip(1).collect();
    let output = Sink::new(
                match cmdline.len() > 0 {
                    true => Some(Pty::new().unwrap()
                                           .spawn_output(cmdline)
                                           .unwrap()),
                    false => None
                });


    let hints = ShCommands::new();

    output.run(TermProc::new(StdinReadKey::new(), &hints),
               EchoPipe{
                        input: NamedReadPipe::new("/tmp/ush".to_string())
                                             .unwrap()
               });
}
