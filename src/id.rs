use std::fmt;
use std::fmt::Write;
use std::iter::FromIterator;

#[derive(Default, PartialEq, Eq, PartialOrd, Debug, Clone)]
pub struct Id {
    bytes: [u8; 20]
}

#[inline]
fn hexencode_byte(inp: u8) -> char {
    match inp {
        0...9 => (inp + 48) as char,
        10...16 => (inp + 87) as char,
        _ => '@'
    }
}

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in &self.bytes {
            f.write_char(hexencode_byte((byte >> 4) & 0x0fu8))?;
            f.write_char(hexencode_byte(byte & 0x0f))?;
        }

        Ok(())
    }
}

impl Id {
    pub fn from(bytes: &[u8]) -> Id {
        let mut id = Id::default();
        id.bytes.copy_from_slice(bytes);
        id
    }

    pub fn to_string(&self) -> String {
        let mut output = Vec::with_capacity(40);
        for byte in &self.bytes {
            output.push(hexencode_byte((byte >> 4) & 0x0fu8));
            output.push(hexencode_byte(byte & 0x0f));
        }

        String::from_iter(output)
    }

    pub fn from_str(target: &str) -> Option<Id> {
        let trimmed = target.trim();
        if trimmed.len() != 40 {
            return None
        }

        let mut id = Id::default();
        let mut cursor = 0;
        for xs in target.bytes() {
            let incoming = match xs {
                48 ... 57 => xs - 48,
                97 ... 102 => xs - 97 + 10,
                65 ... 70 => xs - 65 + 10,
                _ => return None
            };
            let to_shift = (1 + cursor & 1) << 2;
            id.bytes[cursor >> 1] |= incoming << to_shift;
            cursor += 1;
        }

        Some(id)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn id_default_works() {
        let hash : String = super::Id::default().to_string();
        assert_eq!(hash, "0000000000000000000000000000000000000000")
    }

    #[test]
    fn id_from_hash_works_with_lowercase() {
        let id = super::Id::from_str("0123456789abcdef000000000000000000000000").expect("Failed to parse hash.");
        let hash : String = id.to_string();
        assert_eq!(hash, "0123456789abcdef000000000000000000000000")
    }

    #[test]
    fn id_from_hash_works_with_uppercase() {
        let id = super::Id::from_str("0123456789ABCDEF000000000000000000000000").expect("Failed to parse hash.");
        let hash : String = id.to_string();
        assert_eq!(hash, "0123456789abcdef000000000000000000000000")
    }

    #[test]
    fn id_fails_on_bad_length() {
        let result = super::Id::from_str("012345");
        assert_eq!(result, None);
    }

    #[test]
    fn id_fails_on_bad_chars() {
        let oob_g = super::Id::from_str("0123456789abcdefg00000000000000000000000");
        assert_eq!(oob_g, None);

        let oob_g_upper = super::Id::from_str("0123456789abcdefG00000000000000000000000");
        assert_eq!(oob_g_upper, None);

        let oob_colon = super::Id::from_str("0123456789abcdef:00000000000000000000000");
        assert_eq!(oob_colon, None);

        let oob_grave = super::Id::from_str("0123456789abcdef`00000000000000000000000");
        assert_eq!(oob_grave, None);

        let oob_slash = super::Id::from_str("0123456789abcdef/00000000000000000000000");
        assert_eq!(oob_slash, None);

        let oob_at = super::Id::from_str("0123456789abcdef@00000000000000000000000");
        assert_eq!(oob_at, None);
    }
}
