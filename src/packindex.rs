use byteorder::{LittleEndian, BigEndian, ReadBytesExt};
use std::path::{Path, PathBuf};
use error::GitError;
use std::io;
use id::{Id, Cmp};
use std;

impl std::convert::From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError::BadPackfileIndex(e)
    }
}

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
struct IndexEntry {
    pub id: Id,
    pub offset: u64,
    pub crc32: u32
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
    pub fn from (mut stream: Box<std::io::Read>) -> Result<Index, GitError> {
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
        for idx in 0..object_count {
            let offset = offsets_vec[idx];
            let msb_set = offset & 0x80000000;
            if msb_set > 0 {
                large_offset_count += 1;
            }
        }

        let mut large_offsets_vec = vec!(0u64; large_offset_count as usize);
        stream.read_u64_into::<BigEndian>(&mut large_offsets_vec)?;

        let mut entry_vec = Vec::with_capacity(object_count);
        for idx in 0..object_count {
            let id = Id::from_bytes(&oid_bytes_vec.as_slice()[
                (idx * 20)..((idx + 1) * 20)
            ]);

            let offset = offsets_vec[idx];
            let msb_set = offset & 0x80000000;
            let final_offset = if msb_set > 0 {
                large_offsets_vec[(offset & 0x7fffffff) as usize]
            } else {
                offset as u64
            };

            entry_vec.push(IndexEntry {
                id: id,
                offset: final_offset,
                crc32: crc_vec[idx]
            });
        }

        let mut packfile_checksum = [0u8; 20];
        stream.read_exact(&mut packfile_checksum)?;

        let mut checksum = [0u8; 20];
        stream.read_exact(&mut checksum)?;

        Ok(Index {
            fanout: Fanout(fanout),
            objects: entry_vec,
            packfile_checksum: packfile_checksum,
            checksum: checksum
        })
    }

    pub fn get_bounds (&self, id: &Id) -> Option<(u64, u64)> {
        let as_bytes = id.as_slice();
        let mut lo = if as_bytes[0] > 0 {
            self.fanout.0[(as_bytes[0] - 1) as usize]
        } else {
            0
        };
        let mut hi = self.fanout.0[as_bytes[0] as usize];
        let mut middle: usize;

        loop {
            middle = ((lo + hi) >> 1) as usize;
            if middle > self.objects.len() {
                return None
            }

            match id.compare(&self.objects[middle].id) {
                Cmp::Lesser => {
                    hi = middle as u32;
                },
                Cmp::Greater => {
                    lo = (middle + 1) as u32;
                },
                Cmp::Same => {
                    return Some((
                        self.objects[middle].offset,
                        self.objects[middle + 1].offset
                    ));
                }
            };

            if (lo >= hi) {
                break
            }
        }

        return None
    }
}
