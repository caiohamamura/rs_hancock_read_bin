#![feature(float_to_from_bytes)]
extern crate byteorder;

use std::cell::RefCell;
use std::convert::TryInto;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, SeekFrom};

const BUFFER_SIZE: usize = 3000000;

pub trait FromBytes {
    type Item;
    fn from_ne_bytes(vec: Vec<u8>) -> Self::Item;
}

impl FromBytes for u32 {
    type Item = u32;
    fn from_ne_bytes(vec: Vec<u8>) -> Self::Item {
        let arr: [u8; 4] = vec.as_slice().try_into().unwrap();
        Self::Item::from_ne_bytes(arr)
    }
}

impl FromBytes for f32 {
    type Item = f32;
    fn from_ne_bytes(vec: Vec<u8>) -> Self::Item {
        let arr: [u8; 4] = vec.as_slice().try_into().unwrap();
        unsafe {std::mem::transmute::<[u8; 4], f32>(arr)}
    }
}

impl FromBytes for u8 {
    type Item = u8;
    fn from_ne_bytes(vec: Vec<u8>) -> Self::Item {
        let arr: [u8; 1] = vec.as_slice().try_into().unwrap();
        arr[0]
    }
}

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
        Ok(result)
    }

    fn read_metadata(&mut self) -> Result<(), io::Error> {
        self.reader.seek(SeekFrom::End(-(4 + 3 * 8)))?;
        let mut buffer8 = [0u8; 8];
        let mut buffer = [0u8; 4];
        self.reader.read(&mut buffer8)?;
        self.xoff = f64::from_ne_bytes(buffer8);
        self.reader.read(&mut buffer8)?;
        self.yoff = f64::from_ne_bytes(buffer8);
        self.reader.read(&mut buffer8)?;
        self.zoff = f64::from_ne_bytes(buffer8);
        self.reader.read(&mut buffer)?;
        self.n_beams = u32::from_ne_bytes(buffer) as usize;
        self.reader.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    fn read_bytes<T>(&mut self) -> T
    where 
        T: Copy
    {
        let size_of_t = std::mem::size_of::<T>();
        let mut buff_slice = vec![0u8; size_of_t];
        self.reader
            .read(&mut buff_slice)
            .unwrap_or_else(|err| panic!("Can't read file anymore: {}", err));
        let ptr: *mut T = unsafe { std::mem::transmute(&buff_slice) };
        unsafe { *ptr }
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

/*

pub struct HancockReaderInMemory {
    buffer: Vec<u8>,
    buffer_pos: usize,
    path: String,
    pub n_beams: usize,
    pub current_beam: usize,
    pub xoff: f64,
    pub yoff: f64,
    pub zoff: f64,
}

impl HancockReaderInMemory {
    pub fn new(path: String) -> Result<HancockReaderInMemory, io::Error> {
        let path2 = path.clone();
        let mut f = File::open(path2)?;

        let mut result = HancockReaderInMemory {
            buffer: vec![],
            n_beams: 0,
            current_beam: 0,
            buffer_pos: 0,
            xoff: 0.0,
            yoff: 0.0,
            zoff: 0.0,
            path: path.clone(),
        };
        result.read_metadata(&mut f)?;
        Ok(result)
    }

    pub fn load(&mut self) -> Result<(), io::Error> {
        let mut f = File::open(self.path.clone())?;
        f.read_to_end(&mut self.buffer)?;
        Ok(())
    }

    pub fn read_metadata(&mut self, file: &mut File) -> Result<(), io::Error> {
        file.seek(SeekFrom::End(-(4 + 3 * 8)))?;
        let mut buffer8 = [0u8; 8];
        let mut buffer = [0u8; 4];
        file.read(&mut buffer8)?;
        self.xoff = f64::from_ne_bytes(buffer8);
        file.read(&mut buffer8)?;
        self.yoff = f64::from_ne_bytes(buffer8);
        file.read(&mut buffer8)?;
        self.zoff = f64::from_ne_bytes(buffer8);
        file.read(&mut buffer)?;
        self.n_beams = u32::from_ne_bytes(buffer) as usize;
        file.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    fn read_f32(&mut self) -> f32 {
        let buff_slice: [u8; 4] = self.buffer[self.buffer_pos..self.buffer_pos + 4]
            .try_into()
            .expect("Slice of wrong size");
        self.buffer_pos += 4;
        f32::from_ne_bytes(buff_slice)
    }

    fn read_u32(&mut self) -> u32 {
        let buff_slice: [u8; 4] = self.buffer[self.buffer_pos..self.buffer_pos + 4]
            .try_into()
            .expect("Slice of wrong size");
        self.buffer_pos += 4;
        u32::from_ne_bytes(buff_slice)
    }

    fn read_u8(&mut self) -> u8 {
        let buff_slice: [u8; 1] = self.buffer[self.buffer_pos..self.buffer_pos + 1]
            .try_into()
            .expect("Slice of wrong size");
        self.buffer_pos += 1;
        u8::from_ne_bytes(buff_slice)
    }
}

impl Iterator for HancockReaderInMemory {
    type Item = HancockDataRow;

    fn next(&mut self) -> Option<HancockDataRow> {
        if self.current_beam == self.n_beams {
            return None;
        }
        let mut result = HancockDataRow {
            zen: self.read_f32(),
            az: self.read_f32(),
            x: self.read_f32(),
            y: self.read_f32(),
            z: self.read_f32(),
            shot_n: self.read_u32(),
            n_hits: self.read_u8(),
            r: vec![],
            refl: vec![],
        };

        for _ in 0..result.n_hits as usize {
            result.r.push(self.read_f32());
            result.refl.push(self.read_f32());
        }
        self.current_beam += 1;
        Some(result)
    }
} */
