use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom},
    thread,
    time::Duration,
};

pub struct EveReader {
    reader: BufReader<File>,
    buffer: String,
}

impl EveReader {
    pub fn new(path: &str) -> Result<Self> {
        let mut file = File::open(path)
            .with_context(|| format!("impossibile aprire il file {}", path))?;

        file.seek(SeekFrom::End(0))
            .with_context(|| format!("impossibile fare seek su {}", path))?;

        Ok(Self {
            reader: BufReader::new(file),
            buffer: String::new(),
        })
    }

    pub fn next_json(&mut self) -> Result<Value> {
        loop {
            self.buffer.clear();

            let bytes = self
                .reader
                .read_line(&mut self.buffer)
                .context("errore durante la lettura di eve.json")?;

            if bytes == 0 {
                thread::sleep(Duration::from_millis(200));
                continue;
            }

            let trimmed = self.buffer.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(trimmed) {
                Ok(value) => return Ok(value),
                Err(_) => continue,
            }
        }
    }
}