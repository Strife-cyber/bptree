use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub const PAGE_SIZE: usize = 4096;

pub struct Pager {
    file: File,
    pub num_pages: u32,
}

impl Pager {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let file_len = file.metadata()?.len();
        // Round up so a partial trailing page is still counted and safe to read.
        let mut num_pages = ((file_len + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as u32;

        if num_pages == 0 {
            // Initialize meta page (page 0)
            let empty_page = vec![0; PAGE_SIZE];
            file.write_all(&empty_page)?;
            file.sync_all()?;
            num_pages = 1;
        }

        Ok(Self { file, num_pages })
    }

    pub fn reset(&mut self) -> std::io::Result<()> {
        self.file.set_len(0)?;
        self.file.rewind()?;
        let empty_page = vec![0; PAGE_SIZE];
        self.file.write_all(&empty_page)?;
        self.file.sync_all()?;
        self.num_pages = 1;
        Ok(())
    }

    pub fn read_page(&mut self, page_num: u32) -> std::io::Result<Vec<u8>> {
        let mut buffer = vec![0u8; PAGE_SIZE];
        let offset = (page_num as u64) * (PAGE_SIZE as u64);
        self.file.seek(SeekFrom::Start(offset))?;
        // Use read() instead of read_exact() to gracefully handle short/partial pages.
        let mut total_read = 0;
        while total_read < PAGE_SIZE {
            match self.file.read(&mut buffer[total_read..]) {
                Ok(0) => break, // EOF – remaining bytes stay zero-padded
                Ok(n) => total_read += n,
                Err(e) => return Err(e),
            }
        }
        Ok(buffer)
    }

    pub fn write_page(&mut self, page_num: u32, data: &[u8]) -> std::io::Result<()> {
        if data.len() > PAGE_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Data exceeds page size",
            ));
        }

        let mut buffer = vec![0u8; PAGE_SIZE];
        buffer[..data.len()].copy_from_slice(data); // Pad with zeroes

        self.file.seek(SeekFrom::Start((page_num as usize * PAGE_SIZE) as u64))?;
        self.file.write_all(&buffer)?;
        self.file.sync_data()?;
        
        let new_num_pages = page_num + 1;
        if new_num_pages > self.num_pages {
             self.num_pages = new_num_pages;
        }
        
        Ok(())
    }

    /// Flush all OS-buffered writes to disk. Call this during graceful shutdown
    /// to ensure no partially-written pages are left on disk.
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.file.sync_all()
    }

    pub fn allocate_page(&mut self) -> u32 {
        let new_page_num = self.num_pages;
        self.num_pages += 1;
        // Pre-allocate the space with 0s.
        let empty_page = vec![0; PAGE_SIZE];
        self.write_page(new_page_num, &empty_page).unwrap();
        new_page_num
    }
}
