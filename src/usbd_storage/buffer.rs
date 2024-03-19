use core::{cmp::min, convert::Infallible, fmt::Debug};

use defmt::Format;

pub struct Buffer<'a> {
    inner: &'a mut [u8],
    rpos: usize, // next byte to read from
    wpos: usize, // next byte to write into
}

impl<'a> Buffer<'a> {
    pub fn new(inner: &'a mut [u8]) -> Self {
        Buffer {
            inner,
            rpos: 0,
            wpos: 0,
        }
    }

    pub fn available_read(&self) -> usize {
        self.wpos - self.rpos
    }

    pub async fn write<E: Debug + Format>(
        &mut self,
        f: &(impl Reader<Error = E> + ?Sized),
    ) -> Result<usize, E> {
        if self.wpos == self.rpos {
            self.clean();
        }
        f.read(&mut self.inner[self.wpos..]).await.map(|count| {
            self.wpos += count;
            count
        })
    }

    pub async fn write_mut<E: Debug + Format>(
        &mut self,
        f: &mut (impl ReaderMut<Error = E> + ?Sized),
    ) -> Result<usize, E> {
        if self.wpos == self.rpos {
            self.clean();
        }
        f.read(&mut self.inner[self.wpos..]).await.map(|count| {
            self.wpos += count;
            count
        })
    }

    pub async fn read<E>(&mut self, f: &mut (impl Writer<Error = E> + ?Sized)) -> Result<usize, E> {
        let boundary = self.rpos + self.available_read();
        f.write(&self.inner[self.rpos..boundary])
            .await
            .map(|count| {
                self.rpos += count;
                count
            })
    }

    pub fn clean(&mut self) {
        self.rpos = 0;
        self.wpos = 0;
    }
}

pub trait Writer {
    type Error;
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
}

pub trait Reader {
    type Error;
    async fn read(&self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

pub trait ReaderMut {
    type Error;
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

impl Writer for [u8] {
    type Error = Infallible;
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let len = min(self.len(), buf.len());
        self[..len].copy_from_slice(&buf[..len]);
        Ok(len)
    }
}

impl Reader for [u8] {
    type Error = Infallible;
    async fn read(&self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let len = min(self.len(), buf.len());
        buf[..len].copy_from_slice(&self[..len]);
        Ok(len)
    }
}
