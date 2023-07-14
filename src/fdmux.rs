extern crate libc;

use libc::*;
use std::os::unix::prelude::AsRawFd;
use std::os::unix::prelude::RawFd;
use std::io::Read;
use std::io::Error;
use std::io::ErrorKind;

#[allow(dead_code)]
pub enum Action { Read, Write }

impl Action {
    fn poll_flag(&self) -> c_short {
        match self {
            Action::Read => libc::POLLIN,
            Action::Write => libc::POLLOUT
        }
    }

    #[allow(dead_code)]
    fn open_flag(&self) -> c_int {
        match self {
            Action::Read => libc::O_RDONLY,
            Action::Write => libc::O_WRONLY
        }
    }
}

pub struct FdMux {
    pub fds : Vec<pollfd>
}

impl FdMux {
    pub fn new(size : usize) -> Self {
        FdMux { fds: Vec::with_capacity(size) }
    }

    pub fn add<T: AsRawFd>(mut self, src: &T, mode: Action) -> FdMux {
        self.fds.push(pollfd { fd: src.as_raw_fd(),
                               events: mode.poll_flag(),
                               revents: 0 });
        self
    }
}

impl Read for FdMux {
    fn read(&mut self, buff: &mut [u8]) -> Result<usize, Error> {
        println!("pollset");
        let ret = unsafe { poll(self.fds.as_mut_ptr(), self.fds.len() as u64, -1) };
        if ret > 0 {
            for desc in &self.fds {
                println!("{} {} {}", desc.fd, desc.events, desc.revents);
                if desc.revents & desc.events > 0 {
                    unsafe {
                        let len = read(desc.fd, buff.as_mut_ptr() as *mut c_void, buff.len());
                        if len > 0 {
                            return Ok(len as usize);
                        } else {
                            return Err(Error::new(ErrorKind::Other,
                                                  format!("Can't read on fd {}", desc.fd)));
                        }
                    }
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
                println!("{}", fd);
                if fd > 0 {
                    return Ok(NamedReadPipe{fd: fd, name: name});
                }
            }
    
            return Err(Error::new(ErrorKind::PermissionDenied, "Can't open fifo"));
        }
    }
}

impl Read for NamedReadPipe {
    fn read(&mut self, buff: &mut [u8]) -> Result<usize, Error> {
        unsafe {
            let n = read(self.fd, buff.as_mut_ptr() as *mut c_void, buff.len());
            if n > 0 {
                return Ok(n as usize);
            }
            return Err(Error::new(ErrorKind::UnexpectedEof, "Can't read named pipe"));
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
