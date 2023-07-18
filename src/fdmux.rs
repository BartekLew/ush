extern crate libc;

use libc::*;
use std::os::unix::prelude::AsRawFd;
use std::os::unix::prelude::RawFd;
use std::io::Read;
use std::io::Write;
use std::io::Error;
use std::io::ErrorKind;

pub trait ReadStr {
    fn read_str(&mut self) -> Result<String, Error>;
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

    pub fn pass_to<W:Write>(mut self, mut out: W) {
        loop {
            match self.read_str() {
                Ok(s) => {
                    if s.len() > 0 {
                        out.write(s.as_bytes()).unwrap();
                        out.flush().unwrap();
                    }
                }, Err(_) => {return ();}
            }
        }
    }
}

impl <'a> ReadStr for FdMux<'a> {
    fn read_str(&mut self) -> Result<String,Error> {
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

        return Err(Error::new(ErrorKind::Other, "poll() syscall failed"));
    }
}

pub struct NamedReadPipe {
    pub fd: RawFd,
    pub name: String
}

impl NamedReadPipe {
    pub fn new(name: String) -> Result<Self,Error> {
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
    
            return Err(Error::new(ErrorKind::PermissionDenied, "Can't open fifo"));
        }
    }
}

impl ReadStr for NamedReadPipe {
    fn read_str(&mut self) -> Result<String, Error> {
        let mut buff: [u8;1024] = [0;1024];
        unsafe {
            let n = read(self.fd, buff.as_mut_ptr() as *mut c_void, buff.len());
            if n > 0 {
                return Ok(String::from(std::str::from_utf8(&buff[0..n as usize]).unwrap()));
            }
            return Err(Error::new(ErrorKind::UnexpectedEof, "Can't read named pipe"));
        }
    }
}

impl ReadStr for std::io::Stdin {
    fn read_str(&mut self) -> Result<String, Error> {
        let mut buff: [u8; 10] = [0;10];
        match self.read(&mut buff) {
            Ok(n) => Ok(String::from(std::str::from_utf8(&buff[0..n]).unwrap())),
            Err(e) => Err(e)
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

