use std::{error::Error, io::{Write, Read}, time::Duration, ffi::{CString, CStr}, hint, mem::MaybeUninit};

use ctru::prelude::*;

/// Wraps addrinfo to free it when out of scope.
struct AddrInfoWrapper(*mut libc::addrinfo);

impl Drop for AddrInfoWrapper {
    fn drop(&mut self) {
        // SAFETY: This is a necessary free call for C library API. When using
        // AddrInfoWrapper, we ensure that the contents are valid.
        unsafe { libc::freeaddrinfo(self.0) };
    }
}

/// Perform an IPv4 DNS lookup.
fn dns_lookup(hostname: &str) -> Result<Vec<[u8; 4]>, Box<dyn Error>> {
    // convert hostname to c string
    let hostname = CString::new(hostname)?;
    // ask for a TCP IPv4 address
    let mut hints = libc::addrinfo {
        ai_family: libc::AF_INET,
        ai_socktype: libc::SOCK_STREAM,
        ai_flags: libc::AI_PASSIVE,
        ai_protocol: 0,
        ai_canonname: std::ptr::null_mut(),
        ai_addr: std::ptr::null_mut(),
        ai_addrlen: 0,
        ai_next: std::ptr::null_mut(),
    };
    // SAFETY: This is marked as unsafe because of 1) the call to a C function
    // and 2) assuming a MaybeUninit value is initialized. For 1), we know it is
    // safe because we follow the requirements for the parameters, and for 2),
    // we know it is safe because the list must be initialized if getaddrinfo
    // returns 0.
    let list = unsafe {
        // uninitialized list that getaddrinfo will write to
        let mut list = MaybeUninit::<*mut libc::addrinfo>::uninit();
        // get list from getaddrinfo
        if libc::getaddrinfo(hostname.as_ptr(), std::ptr::null(), &mut hints, list.as_mut_ptr()) != 0 {
            // TODO pretty error
            return Err("failed to get address".into());
        }
        // wrap so that it will be dropped by RAII
        AddrInfoWrapper(list.assume_init())
    };

    let mut options = vec![];
    let mut current = list.0;
    // SAFETY: We trust libc that the pointer is able to be dereferenced safely.
    while let Some(info) = unsafe { current.as_ref() } {
        // SAFETY: ditto
        if let Some(addr) = unsafe { info.ai_addr.as_ref() } {
            if i32::from(addr.sa_family) == libc::AF_INET {
                // SAFETY: We trust libc that by setting the family to
                // AF_INET, that the underlying reference is of type
                // sockaddr_in.
                let addr = unsafe {
                    std::mem::transmute::<_, &libc::sockaddr_in>(addr)
                };
                options.push(addr.sin_addr.s_addr.to_ne_bytes());
            }
        }
        // iterate on linked list
        current = info.ai_next;
    }

    Ok(options)
}

fn main_wrapped() -> Result<(), Box<dyn Error>> {
    // need to open socket service to be able to use network
    let _soc = Soc::init()?;

    // attempt to connect to a server
    for option in dns_lookup("google.com")? {
        std::net::TcpStream::connect((option, 80))?;
    }

    Ok(())
}

fn main() {
    let gfx = Gfx::init().unwrap();
    let hid = Hid::init().unwrap();
    let apt = Apt::init().unwrap();

    let _console = ctru::console::Console::init(gfx.top_screen.borrow_mut());

    if let Err(e) = main_wrapped() {
        println!("{:?}", e);
    }

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_held().contains(KeyPad::KEY_START) {
            break;
        }

        gfx.wait_for_vblank();
    }
}
