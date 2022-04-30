use std::io::Read;
use std::str::FromStr;
use std::fmt::{ Display, Write };
use std::convert::{ From, TryFrom };

use crate::errors::{ ErrorKind, Error };

#[derive(Default, Debug, PartialEq, Eq, Ord, PartialOrd, Clone, Hash)]
#[repr(transparent)]
pub struct Id {
    bytes: [u8; 20]
}

impl Id {
    pub fn new<T: AsRef<[u8]>>(bytes: T) -> Self {
        let mut id = Self::default();
        id.bytes.copy_from_slice(bytes.as_ref());
        id
    }

    pub fn new_from_ascii_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self, Error> {
        let bytes = bytes.as_ref();
        if bytes.len() < 40 {
            return Err(ErrorKind::BadId.into())
        }

        Ok(from_ascii_bytes(&bytes[0..40])?.into())
    }

    pub fn read_packed_ids<R: Read>(input: &mut R, count: usize) -> crate::errors::Result<Vec<Id>> {
        let mut ids: Vec<Id> = vec![Id::default(); count];

        // Do some crimes here. We need to read in a list of tightly-packed ids, so we pre-allocate
        // the exact number of ids we expect. Then we get a pointer to that list of ids, pun it to
        // a mut u8 slice, and pass it to read_exact.
        let slice = ids.as_mut_slice();
        let ptr: *mut Id = slice.as_mut_ptr();
        let bytes_ptr = ptr as *mut u8;
        input.read_exact(unsafe { std::slice::from_raw_parts_mut(bytes_ptr, count * 20) })?;
        Ok(ids)
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

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.len() < 40 {
            return Err(ErrorKind::BadId.into())
        }

        Ok(from_ascii_bytes(&input.as_bytes()[0..40])?.into())
    }
}

#[inline]
fn from_ascii_bytes(input: &[u8]) -> Result<[u8; 20], Error> {
    let mut output = [0u8; 20];
    for (cursor, xs) in input.iter().enumerate() {
        let incoming = match xs {
            48 ..= 57 => xs - 48,
            97 ..= 102 => xs - 97 + 10,
            65 ..= 70 => xs - 65 + 10,
            _ => return Err(ErrorKind::BadId.into())
        };
        let to_shift = ((1 + cursor) & 1) << 2;
        output[cursor >> 1] |= incoming << to_shift;
    }
    Ok(output)
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
