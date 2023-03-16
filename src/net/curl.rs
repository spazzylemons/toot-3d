use std::{
    cell::RefCell,
    error::Error,
    ffi::{CStr, CString},
    fmt::Display,
    pin::Pin,
};

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
mod c {
    include!(concat!(env!("OUT_DIR"), "/curl.rs"));
}

#[derive(Debug)]
pub struct CurlError(c::CURLcode);

impl Display for CurlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = unsafe { CStr::from_ptr(c::curl_easy_strerror(self.0)) };
        write!(f, "{}", s.to_string_lossy())
    }
}

impl Error for CurlError {}

pub struct Global(());

impl Global {
    pub fn new() -> Self {
        unsafe { c::curl_global_init(c::CURL_GLOBAL_DEFAULT as _) };
        Self(())
    }
}

impl Drop for Global {
    fn drop(&mut self) {
        unsafe { c::curl_global_cleanup() };
    }
}

pub struct Easy {
    // reference to cURL easy session
    curl: *mut c::CURL,
    // pinned write buffer for getting response body
    write_buffer: Pin<Box<RefCell<Vec<u8>>>>,
}

extern "C" fn write_callback(
    ptr: *mut std::ffi::c_char,
    _size: usize,
    nmemb: usize,
    userdata: *mut std::ffi::c_void,
) -> usize {
    let write_buffer = unsafe { &*(userdata as *const RefCell<Vec<u8>>) };
    write_buffer
        .borrow_mut()
        .extend_from_slice(unsafe { std::slice::from_raw_parts(ptr as *const u8, nmemb) });
    nmemb
}

impl Easy {
    pub fn new() -> Self {
        // get curl pointer
        let curl = unsafe { c::curl_easy_init() };
        if curl.is_null() {
            panic!("curl_easy_init() failed");
        }
        // create write buffer
        let write_buffer = Box::pin(RefCell::new(vec![]));
        // use reference to buffer for callback
        unsafe {
            _ = c::curl_easy_setopt(
                curl,
                c::CURLoption_CURLOPT_WRITEFUNCTION,
                write_callback
                    as extern "C" fn(
                        *mut std::ffi::c_char,
                        usize,
                        usize,
                        *mut std::ffi::c_void,
                    ) -> usize,
            );
            _ = c::curl_easy_setopt(
                curl,
                c::CURLoption_CURLOPT_WRITEDATA,
                write_buffer.as_ref().get_ref(),
            );
        }
        Self { curl, write_buffer }
    }

    pub fn no_verify(&self) -> Result<(), Box<dyn Error>> {
        let res = unsafe {
            c::curl_easy_setopt(
                self.curl,
                c::CURLoption_CURLOPT_SSL_VERIFYPEER,
                0 as std::ffi::c_long,
            )
        };
        if res != c::CURLcode_CURLE_OK {
            return Err(Box::new(CurlError(res)));
        }
        let res = unsafe {
            c::curl_easy_setopt(
                self.curl,
                c::CURLoption_CURLOPT_SSL_VERIFYHOST,
                0 as std::ffi::c_long,
            )
        };
        if res != c::CURLcode_CURLE_OK {
            return Err(Box::new(CurlError(res)));
        }
        Ok(())
    }

    pub fn url(&self, url: &str) -> Result<(), Box<dyn Error>> {
        let url = CString::new(url)?;
        let res =
            unsafe { c::curl_easy_setopt(self.curl, c::CURLoption_CURLOPT_URL, url.as_ptr()) };
        if res != c::CURLcode_CURLE_OK {
            return Err(Box::new(CurlError(res)));
        }
        Ok(())
    }

    pub fn bearer(&self, bearer: Option<&str>) -> Result<(), Box<dyn Error>> {
        let res = if let Some(bearer) = bearer {
            let bearer = CString::new(bearer)?;
            let res = unsafe {
                c::curl_easy_setopt(
                    self.curl,
                    c::CURLoption_CURLOPT_HTTPAUTH,
                    (1 << 6) as std::ffi::c_long,
                )
            };
            if res != c::CURLcode_CURLE_OK {
                return Err(Box::new(CurlError(res)));
            }
            unsafe {
                c::curl_easy_setopt(
                    self.curl,
                    c::CURLoption_CURLOPT_XOAUTH2_BEARER,
                    bearer.as_ptr(),
                )
            }
        } else {
            let res = unsafe {
                c::curl_easy_setopt(
                    self.curl,
                    c::CURLoption_CURLOPT_HTTPAUTH,
                    0 as std::ffi::c_long,
                )
            };
            if res != c::CURLcode_CURLE_OK {
                return Err(Box::new(CurlError(res)));
            }
            unsafe {
                c::curl_easy_setopt(
                    self.curl,
                    c::CURLoption_CURLOPT_XOAUTH2_BEARER,
                    std::ptr::null::<std::ffi::c_void>(),
                )
            }
        };
        if res != c::CURLcode_CURLE_OK {
            return Err(Box::new(CurlError(res)));
        }
        Ok(())
    }

    pub fn mime(&self) -> Mime {
        Mime::new(self)
    }

    pub fn perform(&self) -> Result<(), CurlError> {
        self.write_buffer.as_ref().get_ref().borrow_mut().clear();
        let res = unsafe { c::curl_easy_perform(self.curl) };
        if res != c::CURLcode_CURLE_OK {
            return Err(CurlError(res));
        }
        Ok(())
    }

    pub fn perform_with_mime(&self, mime: Mime) -> Result<(), CurlError> {
        unsafe { c::curl_easy_setopt(self.curl, c::CURLoption_CURLOPT_MIMEPOST, mime.mime) };
        let result = self.perform()?;
        unsafe {
            c::curl_easy_setopt(
                self.curl,
                c::CURLoption_CURLOPT_MIMEPOST,
                std::ptr::null::<std::ffi::c_void>(),
            );
            c::curl_easy_setopt(self.curl, c::CURLoption_CURLOPT_POST, 0 as std::ffi::c_long);
        };
        Ok(result)
    }

    pub fn response_code(&self) -> Result<u16, CurlError> {
        let mut result = 0 as std::ffi::c_long;
        let res = unsafe {
            c::curl_easy_getinfo(self.curl, c::CURLINFO_CURLINFO_RESPONSE_CODE, &mut result)
        };
        if res != c::CURLcode_CURLE_OK {
            return Err(CurlError(res));
        }
        Ok(result as _)
    }

    pub fn buffer(&self) -> Vec<u8> {
        let mut result = vec![];
        let mut mine = self.write_buffer.as_ref().get_ref().borrow_mut();
        std::mem::swap(&mut result, &mut mine);
        result
    }

    pub fn escape(&self, s: &str) -> Result<CurlString, CurlError> {
        let raw = if s.is_empty() {
            // if length of the string is 0, curl will try to get the length, but that
            // is a bad idea as the string isn't null-terminated! so instead, we will
            // call strdup ourselves, which is what curl does internally for 0-length strings.
            extern "C" {
                fn strdup(s: *const std::ffi::c_char) -> *mut std::ffi::c_char;
            }

            unsafe { strdup(b"\0".as_ptr()) }
        } else {
            unsafe { c::curl_easy_escape(self.curl, s.as_ptr(), s.len() as _) }
        };
        // if given null pointer, then allocation failed
        if raw.is_null() {
            Err(CurlError(c::CURLcode_CURLE_OUT_OF_MEMORY))
        } else {
            Ok(unsafe { CurlString::take(raw) })
        }
    }
}

impl Drop for Easy {
    fn drop(&mut self) {
        unsafe { c::curl_easy_cleanup(self.curl) };
    }
}

pub struct Mime {
    mime: *mut c::curl_mime,
}

impl Mime {
    pub fn new(easy: &Easy) -> Self {
        let mime = unsafe { c::curl_mime_init(easy.curl) };
        if mime.is_null() {
            panic!("curl_mime_init() failed");
        }
        Self { mime }
    }

    pub fn add_part(&self, name: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let name = CString::new(name)?;
        let part = unsafe { c::curl_mime_addpart(self.mime) };
        if part.is_null() {
            panic!("curl_mime_addpart() failed");
        }
        unsafe {
            // assume these succeed, for now
            _ = c::curl_mime_name(part, name.as_ptr());
            _ = c::curl_mime_data(part, data.as_ptr(), data.len());
        }
        Ok(())
    }
}

impl Drop for Mime {
    fn drop(&mut self) {
        unsafe { c::curl_mime_free(self.mime) };
    }
}

/// Wraps a string that cURL gave us ownership of, avoiding unnecessary reallocation.
pub struct CurlString {
    raw: *mut std::ffi::c_char,
    len: usize,
}

impl CurlString {
    unsafe fn take(raw: *mut std::ffi::c_char) -> Self {
        extern "C" {
            fn strlen(s: *const std::ffi::c_char) -> usize;
        }
        let len = strlen(raw);
        Self { raw, len }
    }
}

impl AsRef<str> for CurlString {
    fn as_ref(&self) -> &str {
        // assume valid encoding for our cases
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.raw, self.len)) }
    }
}

impl Drop for CurlString {
    fn drop(&mut self) {
        unsafe { c::curl_free(self.raw as *mut _) }
    }
}
