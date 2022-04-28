use std::str::FromStr;
use std::fmt::{ Display, Write };
use std::convert::{ From, TryFrom };

use crate::errors::{ ErrorKind, Error };

#[derive(Default, Debug, PartialEq, Eq, Ord, PartialOrd, Clone, Hash)]
pub struct Id {
    bytes: [u8; 20]
}

impl Id {
    pub fn new<T: AsRef<[u8]>>(bytes: T) -> Id {
        let mut id = Id::default();
        id.bytes.copy_from_slice(bytes.as_ref());
        id
    }
}

#[inline]
fn hexencode_byte(inp: u8) -> char {
    match inp {
        0..=9 => (inp + 48) as char,
        10..=16 => (inp + 87) as char,
        _ => '@'
    }
}

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl FromStr for Id {
    type Err = Error;

    fn from_str(target: &str) -> Result<Self, Self::Err> {
        let trimmed = target.trim();
        if trimmed.len() != 40 {
            return Err(ErrorKind::BadId.into())
        }

        let mut id = Id::default();
        for (cursor, xs) in target.bytes().enumerate() {
            let incoming = match xs {
                48 ..= 57 => xs - 48,
                97 ..= 102 => xs - 97 + 10,
                65 ..= 70 => xs - 65 + 10,
                _ => return Err(ErrorKind::BadId.into())
            };
            let to_shift = ((1 + cursor) & 1) << 2;
            id.bytes[cursor >> 1] |= incoming << to_shift;
        }

        Ok(id)
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for byte in &self.bytes {
            f.write_char(hexencode_byte((byte >> 4) & 0x0fu8))?;
            f.write_char(hexencode_byte(byte & 0x0f))?;
        }

        Ok(())
    }
}

impl From<[u8; 20]> for Id {
    fn from(bytes: [u8; 20]) -> Id {
        Id { bytes }
    }
}

impl TryFrom<&[u8]> for Id {
    fn try_from(bytes: &[u8]) -> Result<Id, Error> {
        if bytes.len() < 20 {
            Err(ErrorKind::BadId.into())
        } else {
            Ok(Id::new(bytes))
        }
    }

    type Error = Error;
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
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
        let result = super::Id::from_str("012345").ok();
        assert_eq!(result, None);
    }

    #[test]
    fn id_fails_on_bad_chars() {
        let oob_g = super::Id::from_str("0123456789abcdefg00000000000000000000000").ok();
        assert_eq!(oob_g, None);

        let oob_g_upper = super::Id::from_str("0123456789abcdefG00000000000000000000000").ok();
        assert_eq!(oob_g_upper, None);

        let oob_colon = super::Id::from_str("0123456789abcdef:00000000000000000000000").ok();
        assert_eq!(oob_colon, None);

        let oob_grave = super::Id::from_str("0123456789abcdef`00000000000000000000000").ok();
        assert_eq!(oob_grave, None);

        let oob_slash = super::Id::from_str("0123456789abcdef/00000000000000000000000").ok();
        assert_eq!(oob_slash, None);

        let oob_at = super::Id::from_str("0123456789abcdef@00000000000000000000000").ok();
        assert_eq!(oob_at, None);
    }
}
