use byteorder::{ BigEndian, ReadBytesExt };
use std::iter::Iterator;
use std;

use crate::id::Id;
use crate::errors::{ Result, ErrorKind };

// index notes:
//      all oids are stored sorted
//      the fanout maps the first byte of an incoming oid to an upper bound index into index entries
// 
//      00a0ddd <- fanout[0] ---+ (lo)
//      00acfff                 |
//      00ad000                 +--------------- this lets us do a bounded binary search for 00acfff
//      00ad001                 |                which gives us the offset in the packfile
//      01bbbbb <- fanout[1] ---+ (hi)

// index format is:
//
//      4 byte magic number (\377tOc)
//      4 byte version number (= 2)
//      256 * 4 fanout table (read last entry to determine N)
//      N * 4 crc32 values
//      N * 4 offset values (31 bit, if 32nd/MSB set number is an offset into large offset table)
//      some number of 8 byte offsets
//      20 byte packfile shasum
//      20 byte shasum of preceding contents
#[derive(Debug)]
pub struct IndexEntry {
    id: Id,
    offset: u64,
    crc32: u32,
    next: usize
}

struct Fanout ([u32; 256]);

impl std::fmt::Debug for Fanout {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Fanout { }")
    }
}

#[derive(Debug)]
pub struct Index {
    fanout: Fanout,
    objects: Vec<IndexEntry>,
    packfile_checksum: [u8; 20],
    checksum: [u8; 20]
}

impl Index {
    pub fn objects(&self) -> &[IndexEntry] {
        self.objects.as_slice()
    }

    pub fn new<T: Iterator<Item=IndexEntry>>(entries: T) -> Result<Self> {
        for entry in entries {
            println!("hi hi");
        }

        Err(ErrorKind::NotImplemented.into())
    }

    pub fn from<T: std::io::Read>(mut stream: T) -> Result<Self> {
        let mut magic = [0u8; 4];
        stream.read_exact(&mut magic)?;
        let mut version = [0u8; 4];
        stream.read_exact(&mut version)?;
        let mut fanout = [0u32; 256];
        stream.read_u32_into::<BigEndian>(&mut fanout)?;

        let object_count = fanout[255] as usize;

        let mut oid_bytes_vec = vec!(0u8; object_count * 20);
        stream.read_exact(&mut oid_bytes_vec.as_mut_slice())?;

        let mut crc_vec = vec!(0u32; object_count);
        stream.read_u32_into::<BigEndian>(&mut crc_vec.as_mut_slice())?;

        let mut offsets_vec = vec!(0u32; object_count);
        stream.read_u32_into::<BigEndian>(&mut offsets_vec.as_mut_slice())?;

        let mut large_offset_count = 0;
        for offset in offsets_vec.iter() {
            let msb_set = offset & 0x8000_0000;
            if msb_set > 0 {
                large_offset_count += 1;
            }
        }

        let mut large_offsets_vec = vec!(0u64; large_offset_count as usize);
        stream.read_u64_into::<BigEndian>(&mut large_offsets_vec)?;

        let mut entry_vec = Vec::with_capacity(object_count);
        for idx in 0..object_count {
            let bytes: &[u8] = oid_bytes_vec.as_ref();
            let id = Id::from(&bytes[
                (idx * 20)..((idx + 1) * 20)
            ]);

            let offset = offsets_vec[idx];
            let msb_set = offset & 0x8000_0000;
            let final_offset = if msb_set > 0 {
                large_offsets_vec[(offset & 0x7fff_ffff) as usize]
            } else {
                u64::from(offset)
            };

            entry_vec.push(IndexEntry {
                id,
                offset: final_offset,
                crc32: crc_vec[idx],
                next: 0
            });
        }

        let mut packfile_checksum = [0u8; 20];
        stream.read_exact(&mut packfile_checksum)?;

        let mut checksum = [0u8; 20];
        stream.read_exact(&mut checksum)?;

        let mut entry_sorted: Vec<(usize, &mut IndexEntry)> = entry_vec.iter_mut().enumerate().collect();

        entry_sorted.sort_by_key(|(_, entry)| {
            entry.offset
        });

        let mut idx = 0;
        while idx < entry_sorted.len() - 1{
            entry_sorted[idx].1.next = entry_sorted[idx + 1].0;
            idx += 1;
        }

        Ok(Index {
            fanout: Fanout(fanout),
            objects: entry_vec,
            packfile_checksum,
            checksum
        })
    }

    pub fn get_bounds (&self, id: &Id) -> Option<(u64, u64)> {
        let as_bytes: &[u8] = id.as_ref();
        let mut lo = if as_bytes[0] > 0 {
            self.fanout.0[(as_bytes[0] - 1) as usize]
        } else {
            0
        };
        let mut hi = self.fanout.0[as_bytes[0] as usize];
        let mut middle: usize;

        loop {
            middle = ((lo + hi) >> 1) as usize;
            if middle >= self.objects.len() {
                return None
            }

            match id.partial_cmp(&self.objects[middle].id) {
                Some(xs) => match xs {
                    std::cmp::Ordering::Less => {
                        hi = middle as u32;
                    },
                    std::cmp::Ordering::Greater => {
                        lo = (middle + 1) as u32;
                    },
                    std::cmp::Ordering::Equal => {
                        return Some((
                            self.objects[middle].offset,
                            self.objects[self.objects[middle].next].offset
                        ));
                    }
                },
                None => return None
            }

            if lo >= hi {
                break
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::Index;

    #[test]
    fn can_load() {
        let bytes = include_bytes!("../fixtures/pack_index");

        let idx = Index::from(&mut bytes.as_ref()).expect("bad index");

        let ids: Vec<String> = idx.objects().iter().take(4).map(|xs| xs.id.to_string()).collect();

        assert_eq!(ids, vec!["45b983be36b73c0788dc9cbcb76cbb80fc7bb057", "4b825dc642cb6eb9a060e54bf8d69288fbee4904", "7f1c6706fbf2edcae73bde0ed0731d01d8f23fe6", "872e26b3fbebe64a2a85b271fed6916b964b4fde"]);

        let offsets: Vec<u64> = idx.objects().iter().take(4).map(|xs| xs.offset).collect();
        assert_eq!(offsets, vec![264, 318, 169, 12]);
    }
}
