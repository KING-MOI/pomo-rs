#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Toggle,
    Skip,
    Quit,
}

impl Key {
    pub fn from_byte(b: u8) -> Option<Key> {
        match b {
            b' ' | b'p' => Some(Key::Toggle),
            b's' => Some(Key::Skip),
            b'q' => Some(Key::Quit),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_bytes_to_keys() {
        assert_eq!(Key::from_byte(b' '), Some(Key::Toggle));
        assert_eq!(Key::from_byte(b'p'), Some(Key::Toggle));
        assert_eq!(Key::from_byte(b's'), Some(Key::Skip));
        assert_eq!(Key::from_byte(b'q'), Some(Key::Quit));
        assert_eq!(Key::from_byte(b'x'), None);
    }
}
