//! Little helpers for writing and reading 1.12.1 packet bodies.

use std::io;

/// A packet body under construction.
#[derive(Default, Clone)]
pub struct Body(pub Vec<u8>);

impl Body {
    pub fn new() -> Self {
        Self(Vec::with_capacity(64))
    }
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn i32(&mut self, v: i32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn f32(&mut self, v: f32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn vec3(&mut self, v: [f32; 3]) -> &mut Self {
        self.f32(v[0]).f32(v[1]).f32(v[2])
    }
    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.0.extend_from_slice(b);
        self
    }
    /// A NUL-terminated string.
    pub fn cstr(&mut self, s: &str) -> &mut Self {
        self.0.extend_from_slice(s.as_bytes());
        self.0.push(0);
        self
    }
    /// A `u32` length (counting the NUL) followed by the string and its NUL, as chat text goes.
    pub fn len_str(&mut self, s: &str) -> &mut Self {
        self.u32(s.len() as u32 + 1);
        self.cstr(s)
    }
    /// The packed guid form: a mask byte, then only the nonzero bytes.
    pub fn packed_guid(&mut self, guid: u64) -> &mut Self {
        let mut mask = 0u8;
        let mut bytes = [0u8; 8];
        let mut n = 0;
        for (i, b) in guid.to_le_bytes().iter().enumerate() {
            if *b != 0 {
                mask |= 1 << i;
                bytes[n] = *b;
                n += 1;
            }
        }
        self.u8(mask);
        self.bytes(&bytes[..n])
    }
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

/// A cursor over a received body.
pub struct Reader<'a> {
    buf: &'a [u8],
}

fn eof() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "packet body ended early")
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }
    pub fn remaining(&self) -> usize {
        self.buf.len()
    }
    pub fn rest(&self) -> &'a [u8] {
        self.buf
    }
    pub fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        if self.buf.len() < n {
            return Err(eof());
        }
        let (head, tail) = self.buf.split_at(n);
        self.buf = tail;
        Ok(head)
    }
    pub fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn f32(&mut self) -> io::Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn array<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        Ok(self.take(N)?.try_into().unwrap())
    }
    pub fn cstr(&mut self) -> io::Result<String> {
        let end = self.buf.iter().position(|&b| b == 0).ok_or_else(eof)?;
        let s = String::from_utf8_lossy(&self.buf[..end]).into_owned();
        self.buf = &self.buf[end + 1..];
        Ok(s)
    }
    pub fn packed_guid(&mut self) -> io::Result<u64> {
        let mask = self.u8()?;
        let mut bytes = [0u8; 8];
        for (i, b) in bytes.iter_mut().enumerate() {
            if mask & (1 << i) != 0 {
                *b = self.u8()?;
            }
        }
        Ok(u64::from_le_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_guid_round_trips() {
        for guid in [0u64, 1, 0x1234, 0xF130_0000_0000_0042, u64::MAX] {
            let mut b = Body::new();
            b.packed_guid(guid);
            let v = b.into_vec();
            assert_eq!(Reader::new(&v).packed_guid().unwrap(), guid);
        }
    }
}
