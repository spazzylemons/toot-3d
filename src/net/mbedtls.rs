use std::{mem::MaybeUninit, ffi::{CString, CStr}, error::Error, pin::Pin, rc::Rc, io::{Read, ErrorKind, Write}, fmt::Display, marker::PhantomData};

use ctru::services::soc::Soc;

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
mod c {
    include!(concat!(env!("OUT_DIR"), "/mbedtls.rs"));
}

#[derive(Debug)]
pub struct MbedTlsError(std::ffi::c_int);

impl Error for MbedTlsError {}

impl Display for MbedTlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = [0; 100];
        let s = unsafe {
            c::mbedtls_strerror(self.0, buf.as_mut_ptr(), buf.len());
            CStr::from_ptr(buf.as_ptr()).to_string_lossy()
        };
        write!(f, "{}", s)
    }
}

extern "C" fn rng_callback(_ptr: *mut std::ffi::c_void, buf: *mut u8, len: usize) -> std::ffi::c_int {
    let r = unsafe { libc::getrandom(buf as _, len, 0) };
    if r as usize != len {
        return -1;
    }
    0
}

pub struct Config(c::mbedtls_ssl_config);

impl Config {
    pub fn new() -> Self {
        unsafe {
            let mut conf = MaybeUninit::uninit();
            c::mbedtls_ssl_config_init(conf.as_mut_ptr());
            Self(conf.assume_init())
        }
    }

    pub fn tls_client_defaults(&mut self) -> Result<(), MbedTlsError> {
        let r = unsafe {
            c::mbedtls_ssl_config_defaults(
                &mut self.0,
                c::MBEDTLS_SSL_IS_CLIENT as _,
                c::MBEDTLS_SSL_TRANSPORT_STREAM as _,
                c::MBEDTLS_SSL_PRESET_DEFAULT as _,
            )
        };

        if r != 0 {
            return Err(MbedTlsError(r));
        }

        Ok(())
    }

    pub fn auth_mode_optional(&mut self) {
        unsafe {
            c::mbedtls_ssl_conf_authmode(
                &mut self.0,
                c::MBEDTLS_SSL_VERIFY_OPTIONAL as _,
            );
        }
    }

    pub fn set_rng(&mut self) {
        unsafe {
            c::mbedtls_ssl_conf_rng(&mut self.0, Some(rng_callback), std::ptr::null_mut());
        }
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        unsafe {
            c::mbedtls_ssl_config_free(&mut self.0);
        }
    }
}

pub struct Ssl<'a> {
    wrapped: c::mbedtls_ssl_context,
    // pins keep the pointers from moving
    _config: Pin<Rc<Config>>,
    _net: Pin<Box<Net<'a>>>,
}

impl<'a> Ssl<'a> {
    pub fn new(config: Pin<Rc<Config>>, mut net: Pin<Box<Net<'a>>>) -> Result<Self, MbedTlsError> {
        let mut ssl_context = unsafe {
            let mut ssl_context = MaybeUninit::uninit();
            c::mbedtls_ssl_init(ssl_context.as_mut_ptr());
            ssl_context.assume_init()
        };

        let ssl_config = &Pin::into_inner(config.as_ref()).0;
        let r = unsafe { c::mbedtls_ssl_setup(&mut ssl_context, ssl_config) };

        if r != 0 {
            unsafe { c::mbedtls_ssl_free(&mut ssl_context) };
            return Err(MbedTlsError(r));
        }

        let net_context = &mut Pin::into_inner(net.as_mut()).0;
        unsafe {
            c::mbedtls_ssl_set_bio(
                &mut ssl_context,
                net_context as *mut _ as _,
                Some(c::mbedtls_net_send),
                Some(c::mbedtls_net_recv),
                None,
            );
        }

        Ok(Self { wrapped: ssl_context, _config: config, _net: net })
    }

    pub fn set_hostname(&mut self, hostname: &str) -> Result<(), Box<dyn Error>> {
        let hostname = CString::new(hostname)?;
        let r = unsafe {
            c::mbedtls_ssl_set_hostname(&mut self.wrapped, hostname.as_ptr())
        };

        if r != 0 {
            return Err(Box::new(MbedTlsError(r)));
        }

        Ok(())
    }

    pub fn handshake(&mut self) -> Result<(), MbedTlsError> {
        let r = unsafe {
            c::mbedtls_ssl_handshake(&mut self.wrapped)
        };

        if r != 0 {
            return Err(MbedTlsError(r));
        }

        Ok(())
    }
}

impl<'a> Drop for Ssl<'a> {
    fn drop(&mut self) {
        unsafe {
            c::mbedtls_ssl_close_notify(&mut self.wrapped);
            c::mbedtls_ssl_free(&mut self.wrapped);
        }
    }
}

impl<'a> Read for Ssl<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let r = unsafe {
            c::mbedtls_ssl_read(&mut self.wrapped, buf.as_mut_ptr(), buf.len())
        };

        if r < 0 {
            return Err(std::io::Error::new(ErrorKind::Other, MbedTlsError(r)));
        }

        return Ok(r as _)
    }
}

impl<'a> Write for Ssl<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let r = unsafe {
            c::mbedtls_ssl_write(&mut self.wrapped, buf.as_ptr(), buf.len())
        };

        if r < 0 {
            return Err(std::io::Error::new(ErrorKind::Other, MbedTlsError(r)));
        }

        return Ok(r as _)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct Net<'a>(c::mbedtls_net_context, PhantomData<&'a ()>);

impl<'a> Net<'a> {
    pub fn new(_soc: &'a Soc) -> Self {
        unsafe {
            let mut net_context = MaybeUninit::uninit();
            c::mbedtls_net_init(net_context.as_mut_ptr());
            Self(net_context.assume_init(), PhantomData)
        }
    }

    pub fn tcp_connect(&mut self, name: &str, port: u16) -> Result<(), Box<dyn Error>> {
        let name = CString::new(name)?;
        let port = CString::new(format!("{port}")).unwrap();
        let r = unsafe {
            c::mbedtls_net_connect(
                &mut self.0,
                name.as_ptr(),
                port.as_ptr(),
                c::MBEDTLS_NET_PROTO_TCP as _,
            )
        };

        if r != 0 {
            return Err(Box::new(MbedTlsError(r)));
        }

        Ok(())
    }
}

impl<'a> Drop for Net<'a> {
    fn drop(&mut self) {
        unsafe {
            c::mbedtls_net_free(&mut self.0);
        }
    }
}
