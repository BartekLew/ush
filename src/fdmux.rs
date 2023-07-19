extern crate libc;
extern crate termios;

use libc::{poll,pollfd,POLLIN,read,unlink,close,mkfifo,open,O_RDWR,c_void};
use termios::*;
use std::os::unix::prelude::AsRawFd;
use std::os::unix::prelude::RawFd;
use std::io::Read;
use std::io::Write;
use std::io::Stdin;

#[derive(Debug)]
pub enum StreamEvent {
    Eof, Error(String)
}

pub trait ReadStr {
    fn read_str(&mut self) -> Result<String, StreamEvent>;
}

pub trait Muxable:AsRawFd+ReadStr {}

pub struct FdMux<'a> {
    inputs : Vec<&'a mut dyn Muxable>,
    fds: Vec<pollfd>
}

impl <'a> FdMux <'a> {
    pub fn new(size : usize) -> Self {
        FdMux { inputs: Vec::with_capacity(size),
                fds: Vec::with_capacity(size) }
    }

    pub fn add<M:Muxable>(mut self, src: &'a mut M) -> FdMux<'a> {
        self.fds.push(pollfd { fd: src.as_raw_fd(),
                               events: POLLIN,
                               revents: 0 });
        self.inputs.push(src);
        self
    }

    pub fn pass_to<W:Write>(&mut self, mut out: W) {
        loop {
            match self.read_str() {
                Ok(s) => {
                    if s.len() > 0 {
                        out.write(s.as_bytes()).unwrap();
                        out.flush().unwrap();
                    }
                },
                Err(StreamEvent::Error(s)) => {
                    println!("Error: {} Terminating.", s);
                    break;
                }
                Err(StreamEvent::Eof) => {
                    break;
                }
            }
        }
    }
}

impl <'a> ReadStr for FdMux<'a> {
    fn read_str(&mut self) -> Result<String,StreamEvent> {
        let ret = unsafe { poll(self.fds.as_mut_ptr(), self.fds.len() as u64, -1) };
        if ret > 0 {
            let len = self.fds.len();
            for i in 0..len {
                let desc = self.fds[i];
                if desc.revents & desc.events > 0 {
                    return self.inputs[i].read_str();
                }
            }
        }

        return Err(StreamEvent::Error("poll() syscall failed".to_string()));
    }
}

pub struct NamedReadPipe {
    pub fd: RawFd,
    pub name: String
}

impl NamedReadPipe {
    pub fn new(name: String) -> Result<Self,StreamEvent> {
        unsafe {
            unlink(name.as_ptr() as *const i8);
            let ret = mkfifo(name.as_ptr() as *const i8, 0o666);
            if ret == 0 {
                // I use RDWR, because opening in O_RDONLY would block
                // on open until someone open another end of pipe.
                let fd = open(name.as_ptr() as *const i8, O_RDWR);
                if fd > 0 {
                    return Ok(NamedReadPipe{fd: fd, name: name});
                }
            }
    
            return Err(StreamEvent::Error("Can't open fifo".to_string()));
        }
    }
}

impl ReadStr for NamedReadPipe {
    fn read_str(&mut self) -> Result<String, StreamEvent> {
        let mut buff: [u8;1024] = [0;1024];
        unsafe {
            let n = read(self.fd, buff.as_mut_ptr() as *mut c_void, buff.len());
            if n > 0 {
                return Ok(String::from(std::str::from_utf8(&buff[0..n as usize]).unwrap()));
            }
            return Err(StreamEvent::Error("Can't read named pipe".to_string()));
        }
    }
}

impl ReadStr for std::io::Stdin {
    fn read_str(&mut self) -> Result<String, StreamEvent> {
        let mut buff: [u8; 10] = [0;10];
        match self.read(&mut buff) {
            Ok(n) => Ok(String::from(std::str::from_utf8(&buff[0..n]).unwrap())),
            Err(e) => Err(StreamEvent::Error(e.to_string()))
        }
    }
}

impl Drop for NamedReadPipe {
    fn drop(&mut self) {
        unsafe { close(self.fd); }
    }
}

impl AsRawFd for NamedReadPipe {
    fn as_raw_fd(&self) -> RawFd { self.fd }
}

impl Muxable for std::io::Stdin {}
impl Muxable for NamedReadPipe {}

pub struct EchoPipe <I: Muxable> {
    pub input: I
}

impl<I:Muxable> AsRawFd for EchoPipe<I> {
    fn as_raw_fd(&self) -> RawFd { self.input.as_raw_fd() }
}

impl<I:Muxable> ReadStr for EchoPipe<I> {
    fn read_str(&mut self) -> Result<String,StreamEvent> {
        self.input.read_str().map(|s| {
                                print!("{}", s);
                                s
                            })
    }
}
impl<I: Muxable> Muxable for EchoPipe<I> {}

pub struct StdinReadKey {
    tos: Termios,
    handle: Stdin
}

impl StdinReadKey {
    pub fn new() -> Self {
        let handle = std::io::stdin();
        let mut tos = Termios::from_fd(handle.as_raw_fd())
                              .expect("This program can't be piped");
        tos.c_lflag &= !(ECHO | ICANON);
        tcsetattr(handle.as_raw_fd(), TCSAFLUSH, &tos).unwrap();

        StdinReadKey { tos: tos, handle: handle }
    }
}

impl AsRawFd for StdinReadKey {
    fn as_raw_fd(&self) -> RawFd { self.handle.as_raw_fd() }
}

impl ReadStr for StdinReadKey {
    fn read_str(&mut self) -> Result<String, StreamEvent> {
        self.handle.read_str()  
    }
}

impl Muxable for StdinReadKey {}

impl Drop for StdinReadKey {
    fn drop(&mut self) {
        self.tos.c_lflag |= ECHO | ICANON;
        tcsetattr(self.as_raw_fd(), TCSAFLUSH, &self.tos).unwrap();
    }
}
