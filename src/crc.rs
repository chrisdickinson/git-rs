use crc::crc32::Digest;
use std::hash::Hasher;
use std::io::Read;

struct CRCReader<R: Read> {
    inner: R,
    digest: Digest
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
        let written = self.inner.read(buf)?;
        if written > 0 {
            self.digest.write(&buf[buf.len() - written ..]);
        } else if buf.len() == 0 {
            self.digest.finish();
        }

        Ok(written)
    }
}
