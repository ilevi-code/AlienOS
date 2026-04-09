use core::fmt::Debug;

pub struct Path {
    pub bytes: [u8],
}

impl Path {
    pub fn new(bytes: &[u8]) -> &Self {
        unsafe { core::mem::transmute(bytes) }
    }
}

impl Debug for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(str::from_utf8(&self.bytes).unwrap_or("<bad path>"))
    }
}

pub struct Components<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Components<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
}

impl<'a> Iterator for Components<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.bytes.len();

        if self.pos >= len {
            return None;
        }

        let mut start = self.pos;

        let end = loop {
            // Find next separator
            let mut end = start;
            while end < len && self.bytes[end] != b'/' {
                end += 1;
            }
            if end != start {
                break end;
            }
            start = end + 1;
        };

        // Move position forward (skip separator if present)
        self.pos = if end < len { end + 1 } else { end };

        Some(&self.bytes[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn starting_with_slash() {
        let mut components = Components::new(b"/foo/bar");
        assert_eq!(components.next(), Some(&b"foo"[..]));
        assert_eq!(components.next(), Some(&b"bar"[..]));
    }

    #[test_case]
    fn ending_with_slash() {
        let mut components = Components::new(b"/foo/bar/");
        assert_eq!(components.next(), Some(&b"foo"[..]));
        assert_eq!(components.next(), Some(&b"bar"[..]));
    }

    #[test_case]
    fn starting_without_slash() {
        let mut components = Components::new(b"foo/bar");
        assert_eq!(components.next(), Some(&b"foo"[..]));
        assert_eq!(components.next(), Some(&b"bar"[..]));
    }

    #[test_case]
    fn skipping_double_slash() {
        let data = b"foo/bar//baz";
        let mut components = Components::new(data);
        assert_eq!(components.next(), Some(&b"foo"[..]));
        assert_eq!(components.next(), Some(&b"bar"[..]));
        assert_eq!(components.next(), Some(&b"baz"[..]));
    }

    #[test_case]
    fn small_components() {
        let mut components = Components::new(b"/aa/b/c");
        assert_eq!(components.next(), Some(&b"aa"[..]));
        assert_eq!(components.next(), Some(&b"b"[..]));
        assert_eq!(components.next(), Some(&b"c"[..]));
    }
}
