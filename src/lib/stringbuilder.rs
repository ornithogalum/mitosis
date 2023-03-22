use std::{
    iter,
    io::Write,
    string::FromUtf8Error
};

const DEFAULT_CAPACITY: usize = 1024;
const MAX_UNICODE_WIDTH: usize = 4;

pub struct Builder(Vec<u8>);

impl Default for Builder {
    fn default() -> Builder {
        let inner = Vec::with_capacity(DEFAULT_CAPACITY);
        Builder(inner)
    }
}

impl Builder {
    pub fn new(size: usize) -> Builder {
        let inner = Vec::with_capacity(size);
        Builder(inner)
    }

    pub fn append<T: ToBytes>(&mut self, buf: T) {
        self.0.write_all(&buf.to_bytes()).unwrap()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn string(self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.0)
    }
}

pub trait ToBytes {
    fn to_bytes(&self) -> Vec<u8>;
}

fn make_copyable_buf(len: usize) -> Vec<u8> {
    iter::repeat(0).take(len).collect::<Vec<u8>>()
}

fn slice_to_vec(s: &[u8]) -> Vec<u8> {
    let mut res = make_copyable_buf(s.len());
    res.copy_from_slice(s);
    res
}

impl ToBytes for String {
    fn to_bytes(&self) -> Vec<u8> {
        slice_to_vec(self.as_bytes())
    }
}

impl<'a> ToBytes for &'a str {
    fn to_bytes(&self) -> Vec<u8> {
        slice_to_vec(self.as_bytes())
    }
}

impl ToBytes for u8 {
    fn to_bytes(&self) -> Vec<u8> {
        vec![*self]
    }
}

impl ToBytes for char {
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = [0; MAX_UNICODE_WIDTH];
        slice_to_vec(self.encode_utf8(&mut buf).as_bytes())
    }
}

impl<'a> ToBytes for &'a [u8] {
    fn to_bytes(&self) -> Vec<u8> {
        slice_to_vec(self)
    }
}