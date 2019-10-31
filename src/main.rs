#![feature(float_to_from_bytes)]
extern crate indicatif;
extern crate threadpool;

use threadpool::ThreadPool;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::convert::TryInto;
use std::fs::File;
use std::io::BufReader;
use std::io;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::thread;
use std::sync::mpsc::sync_channel;


#[allow(dead_code)]
pub struct HancockDataRow {
    pub zen: f32,
    pub az: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub shot_n: u32,
    pub n_hits: u8,
    pub r: Vec<f32>,
    pub refl: Vec<f32>,
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
    pub fn new(path: String) -> Result<HancockReader, std::io::Error> {
        let file = File::open(path)?;
        let mut result = HancockReader {
            reader: BufReader::with_capacity(3000000, file),
            n_beams: 0,
            current_beam: 0,
            xoff: 0.0,
            yoff: 0.0,
            zoff: 0.0,
        };

        result.read_metadata()?;
        Ok(result)
    }

    fn read_metadata(&mut self) -> Result<(), std::io::Error> {
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

    fn read_f32(&mut self) -> f32 {
        let mut buff_slice: [u8; 4] = Default::default();
        self.reader
            .read(&mut buff_slice)
            .unwrap_or_else(|err| panic!("Can't read file anymore: {}", err));
        f32::from_ne_bytes(buff_slice)
    }

    fn read_u32(&mut self) -> u32 {
        let mut buff_slice: [u8; 4] = Default::default();
        self.reader
            .read(&mut buff_slice)
            .unwrap_or_else(|err| panic!("Can't read file anymore: {}", err));
        u32::from_ne_bytes(buff_slice)
    }

    fn read_u8(&mut self) -> u8 {
        let mut buff_slice: [u8; 1] = Default::default();
        self.reader
            .read(&mut buff_slice)
            .unwrap_or_else(|err| panic!("Can't read file anymore: {}", err));
        u8::from_ne_bytes(buff_slice)
    }
}

impl Iterator for HancockReader {
    type Item = HancockDataRow;

    fn next(&mut self) -> Option<HancockDataRow> {
        self.current_beam += 1;
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

        Some(result)
    }
}

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
    pub fn new(path: String) -> Result<HancockReaderInMemory, std::io::Error> {
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

    pub fn load(&mut self) -> Result<(), std::io::Error> {
        let mut f = File::open(self.path.clone())?;
        f.read_to_end(&mut self.buffer)?;
        Ok(())
    }

    pub fn read_metadata(&mut self, file: &mut File) -> Result<(), std::io::Error> {
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
}


use std::path::PathBuf;
use structopt::StructOpt;

/// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    // A flag, true if used in the command line. Note doc comment will
    // be used for the help message of the flag. The name of the
    // argument will be, by default, based on the name of the field.
    /// Activate debug mode
    /// Files to process
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
}

fn main() -> io::Result<()> {
    let pool = ThreadPool::new(2);
    let opt = Opt::from_args();
    let m = MultiProgress::new();
    let sty = ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
        .progress_chars("#>-");

    opt.files.into_iter().for_each(|file_path| {
        let file_path_str = file_path
            .into_os_string()
            .into_string()
            .expect("Error converting the file string");

        let mut f = HancockReader::new(file_path_str.clone())
            .unwrap_or_else(|err| panic!("Cannot open file: {}!", err));

        let pb = m.add(ProgressBar::new((f.n_beams) as u64));
        pb.set_style(sty.clone());
        let _ = pool.execute(move || {
            pb.set_message(&format!("Processing file: {}", file_path_str.clone()));
            pb.set_position(0);
            while let Some(data) = f.next() {
                if f.current_beam % 10000 == 0 {
                    pb.set_position((f.current_beam + 1) as u64);
                }
            }
            pb.set_position(f.n_beams as u64);
            pb.finish_with_message("done!");
        });
    });

    m.join_and_clear().unwrap();

    // and more! See the other methods for more details.
    Ok(())
}
