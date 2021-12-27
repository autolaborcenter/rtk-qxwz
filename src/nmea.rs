pub(crate) struct Buffer<const LEN: usize> {
    buf: [u8; LEN],
    p_read: usize,
    p_star: usize,
    p_write: usize,
}

macro_rules! check {
    ($t:ty; $slice:expr) => {{
        #[inline]
        fn parse_u8(byte: u8) -> Option<u8> {
            match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                b'A'..=b'F' => Some(byte - b'A' + 10),
                _ => None,
            }
        }

        #[inline]
        fn parse_cs(cs: &[u8]) -> Option<$t> {
            let mut sum = 0;
            for c in cs {
                sum = (sum << 4) + parse_u8(*c)? as $t;
            }
            Some(sum)
        }

        parse_cs($slice)
    }};
}

impl<const LEN: usize> Buffer<LEN> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            buf: [0u8; LEN],
            p_read: 0,
            p_star: 0,
            p_write: 0,
        }
    }

    pub fn to_write<'a>(&'a mut self) -> &'a mut [u8] {
        self.move_p_read();
        if self.p_read > 0 {
            let src = self.p_read..self.p_write;
            self.p_write -= self.p_read;
            self.p_star -= self.p_read;
            self.p_read = 0;
            if !src.is_empty() {
                self.buf.copy_within(src, 0);
            }
        }
        &mut self.buf[self.p_write..]
    }

    #[inline]
    pub fn extend(&mut self, n: usize) {
        self.p_write += n;
    }

    pub fn parse<'a>(&'a mut self) -> Option<&'a str> {
        loop {
            if self.p_write == self.p_read {
                return None;
            }
            match self.get(self.p_read) {
                b'$' => match check!(u8; self.get_checksum_or_move_p(2)?) {
                    Some(sum) => {
                        if sum == xor(&self.buf[self.p_read + 1..self.p_star]) {
                            let result = &self.buf[self.p_read..self.p_star + 3];
                            self.p_star += 2;
                            self.p_read = self.p_star;
                            return Some(unsafe { std::str::from_utf8_unchecked(result) });
                        }
                        self.p_read += 1;
                    }
                    None => {
                        self.p_star += 2;
                        self.p_read = self.p_star;
                    }
                },
                _ => self.move_p_read(),
            }
        }
    }

    #[inline]
    fn get(&self, i: usize) -> u8 {
        unsafe { *self.buf.get_unchecked(i) }
    }

    fn move_p_read(&mut self) {
        while self.p_read < self.p_write {
            // 找到起始位
            match self.get(self.p_read) {
                b'#' | b'$' => break,
                _ => self.p_read += 1,
            }
        }
        if self.p_star < self.p_read || self.get(self.p_star) != b'*' {
            self.p_star = self.p_read;
        }
    }

    fn get_checksum_or_move_p<'a>(&'a mut self, n: usize) -> Option<&'a [u8]> {
        while self.p_star < self.p_write {
            // 找到星号
            if self.get(self.p_star) == b'*' {
                let end = self.p_star + n + 1;
                return if end < self.p_write {
                    // 校验位收齐
                    Some(&self.buf[self.p_star + 1..][..n])
                } else {
                    // 校验位不齐，检查是否可能收齐
                    if self.p_read + LEN <= end {
                        self.p_star += 1;
                        self.p_read = self.p_star;
                    }
                    None
                };
            }
            self.p_star += 1;
        }
        // 未找到星号，检查是否可能找到
        if self.p_read + LEN <= self.p_write + n {
            self.p_read += 1;
            self.p_star = self.p_read;
        }
        None
    }
}

#[inline]
fn xor(buf: &[u8]) -> u8 {
    buf.iter().fold(0, |sum, it| sum ^ it)
}

#[test]
fn test_write() {
    const LEN: usize = 512;
    let mut buffer = Buffer {
        buf: [0u8; LEN],
        p_read: 0,
        p_star: 0,
        p_write: 0,
    };
    let buf = buffer.to_write();
    assert_eq!(LEN, buf.len());

    let msg1 = b"123456$GPGGA,060220.00,3959.55874779,N,11619.61828897,E,1,17,1.6,60.1397,M,-9.2862,M,,*42\r\n";
    buf[..msg1.len()].copy_from_slice(msg1);
    buffer.extend(msg1.len());
    assert_eq!(0, buffer.p_read);
    assert_eq!(0, buffer.p_star);
    assert_eq!(msg1.len(), buffer.p_write);

    let len = buffer.to_write().len();
    assert_eq!(LEN - buffer.p_write, len);
}
