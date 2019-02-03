use std::io::{ BufReader, SeekFrom };
use flate2::bufread::DeflateDecoder;
use std::io::prelude::*;
use std;

use crate::delta::{ DeltaDecoder, DeltaDecoderStream, OFS_DELTA, REF_DELTA };
use crate::stores::{ Queryable, StorageSet };
use crate::errors::{ Result, ErrorKind };
use crate::packindex::Index;
use crate::objects::Type;
use crate::id::Id;

pub type GetObject = Fn(&Id) -> Result<Option<(Type, Box<std::io::Read>)>>;

pub struct Store<R> {
    read: Box<Fn() -> Result<R>>,
    index: Index
}

fn build_index(_reader: Box<std::io::Read>) -> Result<Index> {
    Err(ErrorKind::NotImplemented.into())
}

impl<R: std::io::Read + std::io::Seek + 'static> Store<R> {
    pub fn new<C>(func: C, index: Option<Index>) -> Result<Self>
        where C: Fn() -> Result<R> + 'static {

        let idx = match index {
            Some(xs) => xs,
            None => build_index(Box::new(func()?))?
        };

        Ok(Store {
            read: Box::new(func),
            index: idx
        })
    }

    pub fn read_bounds<Q: Queryable> (&self, start: u64, end: u64, backends: &StorageSet<Q>) -> Result<(u8, Box<std::io::Read>)> {
        let handle = (self.read)()?;
        let mut buffered_file = BufReader::new(handle);
        buffered_file.seek(SeekFrom::Start(start))?;

        let stream = buffered_file.take(end - start);

        // type + size bytes
        let mut continuation;
        let mut size_vec = Vec::new();
        let mut byte = [0u8; 1];

        let mut take_one = stream.take(1);
        take_one.read_exact(&mut byte)?;
        let mut original_stream = take_one.into_inner();
        continuation = byte[0] & 0x80;
        let type_flag = (byte[0] & 0x70) >> 4;
        size_vec.push(byte[0] & 0x0f);
        loop {
            if continuation < 1 {
                break
            }

            take_one = original_stream.take(1);
            take_one.read_exact(&mut byte)?;
            original_stream = take_one.into_inner();
            continuation = byte[0] & 0x80;
            size_vec.push(byte[0] & 0x7f); 
        }
        let mut object_stream = original_stream;

        let count = size_vec.len();
        let mut _size = match size_vec.pop() {
            Some(xs) => u64::from(xs),
            None => return Err(ErrorKind::CorruptedPackfile.into())
        };

        while !size_vec.is_empty() {
            let next = match size_vec.pop() {
                Some(xs) => u64::from(xs),
                None => return Err(ErrorKind::CorruptedPackfile.into())
            };
            _size |= next << (4 + 7 * (count - size_vec.len()));
        }

        match type_flag {
            0...4 => {
                let mut zlib_header = [0u8; 2];
                object_stream.read_exact(&mut zlib_header)?;
                Ok((type_flag, Box::new(DeflateDecoder::new(object_stream))))
            },

            OFS_DELTA => {
                let mut take_one = object_stream.take(1);
                take_one.read_exact(&mut byte)?;
                let mut offset = u64::from(byte[0] & 0x7F);
                let mut original_stream = take_one.into_inner();

                while byte[0] & 0x80 > 0 {
                    offset += 1;
                    offset <<= 7;
                    take_one = original_stream.take(1);
                    take_one.read_exact(&mut byte)?;
                    offset += u64::from(byte[0] & 0x7F);
                    original_stream = take_one.into_inner();
                }

                let mut zlib_header = [0u8; 2];
                original_stream.read_exact(&mut zlib_header)?;
                let mut deflate_stream = DeflateDecoder::new(original_stream);
                let mut instructions = Vec::new();
                deflate_stream.read_to_end(&mut instructions)?;

                let (base_type, mut stream) = self.read_bounds(start - offset, start, backends)?;
                let mut base_buf = Vec::new();

                stream.read_to_end(&mut base_buf)?;
                let delta_decoder = DeltaDecoder::new(&instructions, base_buf)?;
                let dd_stream: DeltaDecoderStream = delta_decoder.into();
                Ok((base_type, Box::new(dd_stream)))
            },

            REF_DELTA => {
                let mut ref_bytes = [0u8; 20];
                object_stream.read_exact(&mut ref_bytes)?;
                let id = Id::from(&ref_bytes);

                let mut zlib_header = [0u8; 2];
                object_stream.read_exact(&mut zlib_header)?;
                let mut deflate_stream = DeflateDecoder::new(object_stream);
                let mut instructions = Vec::new();
                deflate_stream.read_to_end(&mut instructions)?;

                let (t, mut base_stream) = match backends.get(&id)? {
                    Some((xs, stream)) => match xs {
                        Type::Commit => (1, stream),
                        Type::Tree => (2, stream),
                        Type::Blob => (3, stream),
                        Type::Tag => (4, stream)
                    },
                    None => return Err(ErrorKind::CorruptedPackfile.into())
                };

                let mut base_buf = Vec::new();

                base_stream.read_to_end(&mut base_buf)?;


                let delta_decoder = DeltaDecoder::new(&instructions, base_buf)?;
                let stream: DeltaDecoderStream = delta_decoder.into();
                Ok((t, Box::new(stream)))
            },

            _ => {
                Err(ErrorKind::BadLooseObject.into())
            }
        }
    }
}
