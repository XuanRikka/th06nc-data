use std::io::{Cursor, Read, Write};

use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crc32fast::hash;

use crate::encrytion::{apply_key, key_gen};

pub const MAIGC: &[u8; 4] = b"PKGL";
/// 需要注意的是这不是一个完整条目的大小，一个条目是变长的
/// 实际的大小是 `ENTRY_LENGTH + name_len`
pub const ENTRY_LENGTH: usize = 32;
pub const VER0102_ENTRY_LENGTH: usize = 32+11;
pub const VER0102_DATA: &[u8; 63] = b"This Game is Curtain Fire Shooting Game by Shrine Maiden.\nZUN\n\n";


#[derive(Debug)]
pub struct Header
{
    magic: [u8; 4],
    index_length: u32,
    pub entries: Vec<Entry>,
}

impl Header {
    pub fn read<R: Read>(inner: &mut R, name: &str) -> Result<Header>
    {
        let mut magic = [0u8; 4];
        inner.read_exact(&mut magic)?;
        if magic != *MAIGC
        {
            return Err(anyhow!("magic错误！"));
        }
        let index_length = inner.read_u32::<LittleEndian>()?;
        let mut index_data = vec![0u8; index_length as usize];
        inner.read_exact(&mut index_data)?;

        let seed = hash(name.as_bytes());
        let key = key_gen(seed);
        apply_key(&mut index_data, &key);

        let mut cur = Cursor::new(&index_data);
        let len = index_data.len() as u64;
        let mut entries = Vec::new();

        while cur.position() < len {
            entries.push(Entry::read(&mut cur)?);
        }

        Ok(Header {
            magic,
            index_length,
            entries
        })
    }
}

#[derive(Debug)]
pub struct Entry
{
    pub flags: u16,
    pub key: u32,
    pub raw_size: u64,
    pub stored_size: u64,
    pub offset: u64,
    pub name_len: u16,
    pub name: String
}

impl Entry {
    pub fn new(
        storage: Storage,
        key: u32,
        raw_size: u64,
        stored_size: u64,
        offset: u64,
        name: impl Into<String>,
    ) -> Entry {
        let name = name.into();
        Entry {
            flags: storage.to_flags(),
            key,
            raw_size,
            stored_size,
            offset,
            name_len: name.len() as u16,
            name,
        }
    }

    pub fn read<R: Read>(inner: &mut R) -> Result<Entry> {
        let flags = inner.read_u16::<LittleEndian>()?;
        let key = inner.read_u32::<LittleEndian>()?;
        let raw_size = inner.read_u64::<LittleEndian>()?;
        let stored_size = inner.read_u64::<LittleEndian>()?;
        let offset = inner.read_u64::<LittleEndian>()?;
        let name_len = inner.read_u16::<LittleEndian>()?;

        let mut buf = vec![0u8; name_len as usize];
        inner.read_exact(&mut buf)?;

        let name = String::from_utf8(buf)?;

        Ok(Entry {
            flags,
            key,
            raw_size,
            stored_size,
            offset,
            name_len,
            name,
        })
    }

    pub fn write<W: Write>(self, inner: &mut W) -> Result<()> {
        inner.write_u16::<LittleEndian>(self.flags)?;
        inner.write_u32::<LittleEndian>(self.key)?;
        inner.write_u64::<LittleEndian>(self.raw_size)?;
        inner.write_u64::<LittleEndian>(self.stored_size)?;
        inner.write_u64::<LittleEndian>(self.offset)?;
        inner.write_u16::<LittleEndian>(self.name_len)?;
        inner.write_all(self.name.as_bytes())?;
        Ok(())
    }

    #[inline]
    pub fn storage(&self) -> Storage
    {
        Storage::from_flags(self.flags)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Storage
{
    Stored,
    Zstd
}

impl Storage {
    #[inline]
    pub const fn from_flags(flags: u16) -> Self {
        if flags & 1 != 0 { Storage::Zstd } else { Storage::Stored }
    }

    #[inline]
    pub const fn to_flags(self) -> u16 {
        match self {
            Storage::Stored => 0,
            Storage::Zstd => 1,
        }
    }
}
