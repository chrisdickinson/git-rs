use std::hash::Hasher;
use crc::crc32::Digest;
use std::io::{ Read, SeekFrom };
use std::fmt::Debug;

struct CRCReader<R: Read> {
    inner: R,
    digest: Digest
}

impl<R: Read + Debug> Debug for CRCReader<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

impl<R: Read> CRCReader<R> {
    pub fn new(inner: R, digest: Digest) -> Self {
        CRCReader {
            inner,
            digest
        }
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn digest(&self) -> &Digest {
        &self.digest
    }
}

impl<R: Read> Read for CRCReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.inner.read(buf)
    }
}
