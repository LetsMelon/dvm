pub enum MemoryLane<'a> {
    ReadOnly(&'a [u8]),
    ReadWrite(&'a mut [u8]),
}

impl<'a> MemoryLane<'a> {
    pub fn size(&self) -> u64 {
        match self {
            MemoryLane::ReadOnly(slice) => slice.len() as u64,
            MemoryLane::ReadWrite(slice) => slice.len() as u64,
        }
    }

    pub fn read(&self, address: usize) -> Result<u8, String> {
        let value = match self {
            MemoryLane::ReadOnly(slice) => slice.get(address),
            MemoryLane::ReadWrite(slice) => slice.get(address),
        };

        value
            .copied()
            .ok_or_else(|| "Read address out of bounds".to_string())
    }
}
