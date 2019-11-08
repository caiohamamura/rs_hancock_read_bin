use std::cell::RefCell;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, SeekFrom};
use std::io::{BufRead};

const BUFFER_SIZE: usize = 3000000;

#[derive(Debug)]
pub struct HancockDataRow {
    pub zen: f32,
    pub az: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub shot_n: u32,
    pub n_hits: u8,
    pub r: RefCell<Vec<f32>>,
    pub refl: RefCell<Vec<f32>>,
}

pub struct HancockReader {
    reader: BufReader<File>,
    pub n_beams: usize,
    pub current_beam: usize,
    pub xoff: f64,
    pub yoff: f64,
    pub zoff: f64,
}

impl HancockReader {
    pub fn new(path: String) -> Result<HancockReader, io::Error> {
        let file = File::open(path)?;
        let mut result = HancockReader {
            reader: BufReader::with_capacity(BUFFER_SIZE, file),
            n_beams: 0,
            current_beam: 0,
            xoff: 0.0,
            yoff: 0.0,
            zoff: 0.0,
        };
        result.read_metadata()?;
        result.reader.fill_buf().unwrap();
        Ok(result)
    }

    pub fn new_with_buffer_capacity(
        path: String,
        buffer_size: usize,
    ) -> Result<HancockReader, io::Error> {
        let file = File::open(path)?;
        let mut result = HancockReader {
            reader: BufReader::with_capacity(buffer_size, file),
            n_beams: 0,
            current_beam: 0,
            xoff: 0.0,
            yoff: 0.0,
            zoff: 0.0,
        };
        result.read_metadata()?;
        result.reader.fill_buf().unwrap();
        Ok(result)
    }

    fn read_metadata(&mut self) -> Result<(), io::Error> {
        self.reader.seek(SeekFrom::End(-(4 + 3 * 8)))?;

        self.xoff = self.read_bytes::<f64>();
        self.yoff = self.read_bytes::<f64>();
        self.zoff = self.read_bytes::<f64>();
        self.n_beams = self.read_bytes::<u32>() as usize;
        self.reader.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    fn read_bytes<T>(&mut self) -> T
    where
        T: Copy,
    {
        let size_of_t = std::mem::size_of::<T>();
        let mut buff_slice = vec![0u8; size_of_t];
        loop {
            let bytes_read = self
                .reader
                .read(&mut buff_slice)
                .unwrap_or_else(|err| panic!("Can't read file anymore: {}", err));

            if bytes_read == size_of_t {
                break;
            }

            self.reader
                .seek(SeekFrom::Current(-(bytes_read as i64)))
                .unwrap();
            self.reader.fill_buf().unwrap();
        }

        unsafe {
            let raw_ptr: *mut T = std::mem::transmute(&buff_slice[0]);
            *raw_ptr
        }
    }
}

impl Iterator for HancockReader {
    type Item = HancockDataRow;

    fn next(&mut self) -> Option<HancockDataRow> {
        self.current_beam += 1;
        if self.current_beam == self.n_beams {
            return None;
        }
        let result = HancockDataRow {
            zen: self.read_bytes::<f32>(),
            az: self.read_bytes::<f32>(),
            x: self.read_bytes::<f32>(),
            y: self.read_bytes::<f32>(),
            z: self.read_bytes::<f32>(),
            shot_n: self.read_bytes::<u32>(),
            n_hits: self.read_bytes::<u8>(),
            r: RefCell::new(vec![]),
            refl: RefCell::new(vec![]),
        };

        for _ in 0..result.n_hits as usize {
            result.r.borrow_mut().push(self.read_bytes::<f32>());
            result.refl.borrow_mut().push(self.read_bytes::<f32>());
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run() {
        let file_path = String::from(
            "E:/Documentos/Doutorado/Edinburgh/P036_entry2_2017_001_170925_095748 - Copia.bin",
        );
        let mut reader: HancockReader = HancockReader::new_with_buffer_capacity(file_path, 3000000)
            .expect("Failed to open reader.");
        println!("Number of beams: {}", reader.n_beams);
        while let Some(data) = reader.next() {
            print!(
                "\r{:.2} Az: {:.2}",
                (100.0 * data.shot_n as f64 / reader.n_beams as f64),
                data.az
            );
            if data.shot_n > reader.n_beams as u32 {
                println!("OMG something very wrong, shot_n: {}\n\n", data.shot_n);
            }
        }
    }
}
