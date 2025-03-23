mod hint;
mod term;
mod autocomp;
use crate::term::*;
use crate::hint::*;
use libc::getpid;
use fdmux::*;
use std::fs::File;
use std::io::{Write, Error,ErrorKind};
use std::time::SystemTime;

struct RecStdout {
    rec_out: File
}

impl RecStdout {
    fn new(filename: String) -> Result<Self, String> {
        match File::create(&*filename) {
            Ok(f) => Ok(RecStdout { rec_out: f }),
            Err(e) => Err(format!("Can't record to {}: {}", filename, e))
        }
    }
}

impl Write for RecStdout {
    fn write(&mut self, buff: &[u8]) -> Result<usize, Error> {
        if let Err(e) = std::io::stdout().write(buff) {
            return Err(e)
        }

        SystemTime::now()
                   .duration_since(std::time::UNIX_EPOCH)
                   .map_err(|e| Error::new(ErrorKind::Interrupted, format!("Can't get timestamp: {}", e)))
                   .and_then(|ts|
                      self.rec_out.write(format!("{}\x02{}\x03", ts.as_secs(),
                                                unsafe { std::str::from_utf8_unchecked(buff) }
                                        ).as_bytes()))
                   .map(|_| buff.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
        std::io::stdout().flush()
                         .and_then(|_| self.rec_out.flush())
    }
}

impl DoCtrlD for RecStdout {
     fn ctrl_d(&mut self) -> bool {
        eprintln!("CTRLD");
        false
    }
}

struct Sink {
    out: Box<dyn DoCtrlD>,
    pipe: Option<IOPipe>
}

impl Sink {
    fn new(pipe: Option<IOPipe>, pipeline_out: Box<dyn DoCtrlD>) -> Self {
        match pipe {
            Some(handle) => Sink { out: pipeline_out, pipe: Some(handle) },
            None => Sink { out: pipeline_out, pipe: None }
        }
    }
    
    fn run<T: Muxable, U: Muxable>(mut self, mut cmdpipe: T, mut inpipe: U)  {
        match self.pipe {
            Some(mut pipe) => {
                let mut second = pipe.clone();
                Topology::new(2)
                         .add(Destination::new(self.out.as_mut(), vec![&mut pipe]))
                         .add(Destination::new(&mut second, vec![&mut cmdpipe, &mut inpipe]))
                         .run();
            }, 
            None => {
                Topology::new(2)
                         .add(Destination::new(self.out.as_mut(),
                              vec![&mut cmdpipe, &mut inpipe]))
                         .run();
            }
        }
    }
}

fn main() {
    let mut cmdline: Vec<String> = std::env::args().skip(1).collect();

    let pipeline_out : Box<dyn DoCtrlD> = {
        if cmdline.len() > 2 && cmdline[0] == "-rec" {
           cmdline.remove(0);
           match RecStdout::new(cmdline.remove(0)) {
               Ok(out) => Box::new(out),
               Err(e) => {
                    eprintln!("{}", e);
                    Box::new(std::io::stdout())
               }
           }
        } else {
           Box::new(std::io::stdout())
        }
    };

    let proc_out = Sink::new(
                match cmdline.len() > 0 {
                    true => Some(Pty::new().unwrap()
                                           .spawn_output(cmdline)
                                           .unwrap()),
                    false => None
                }, pipeline_out);


    let hints = ShCommands::new();

    proc_out.run(TermProc::new(StdinReadKey::new(), &hints),
                    EchoPipe{
                        input: NamedReadPipe::new(format!("/tmp/ush-{}", unsafe{ getpid() }))
                                             .unwrap()
                    });
}
