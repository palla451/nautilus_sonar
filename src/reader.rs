use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Seek, SeekFrom},
    os::unix::fs::MetadataExt,
    thread,
    time::Duration,
};

pub struct EveReader {
    path: String,
    reader: BufReader<File>,
    buffer: String,
    inode: u64,
    last_size: u64,
}

impl EveReader {
    pub fn new(path: &str) -> Result<Self> {
        let mut file = File::open(path)
            .with_context(|| format!("impossibile aprire il file {}", path))?;

        file.seek(SeekFrom::End(0))
            .with_context(|| format!("impossibile fare seek su {}", path))?;

        let metadata = fs::metadata(path)
            .with_context(|| format!("impossibile leggere metadata di {}", path))?;

        Ok(Self {
            path: path.to_string(),
            reader: BufReader::new(file),
            buffer: String::new(),
            inode: metadata.ino(),
            last_size: metadata.len(),
        })
    }

    fn reopen(&mut self, from_start: bool) -> Result<()> {
        let mut file = File::open(&self.path)
            .with_context(|| format!("impossibile riaprire il file {}", self.path))?;

        if from_start {
            file.seek(SeekFrom::Start(0))
                .with_context(|| format!("impossibile fare seek start su {}", self.path))?;
        } else {
            file.seek(SeekFrom::End(0))
                .with_context(|| format!("impossibile fare seek end su {}", self.path))?;
        }

        let metadata = fs::metadata(&self.path)
            .with_context(|| format!("impossibile leggere metadata di {}", self.path))?;

        self.reader = BufReader::new(file);
        self.inode = metadata.ino();
        self.last_size = metadata.len();

        Ok(())
    }

    fn check_rotation_or_truncate(&mut self) -> Result<()> {
        let metadata = fs::metadata(&self.path)
            .with_context(|| format!("impossibile leggere metadata di {}", self.path))?;

        let current_inode = metadata.ino();
        let current_size = metadata.len();

        // File ruotato: inode diverso
        if current_inode != self.inode {
            self.reopen(true)?;
            return Ok(());
        }

        // File troncato: dimensione più piccola rispetto a prima
        if current_size < self.last_size {
            self.reopen(true)?;
            return Ok(());
        }

        self.last_size = current_size;

        Ok(())
    }

    pub fn next_json(&mut self) -> Result<Value> {
        loop {
            self.check_rotation_or_truncate()?;

            self.buffer.clear();

            let bytes = self
                .reader
                .read_line(&mut self.buffer)
                .context("errore durante la lettura di eve.json")?;

            if bytes == 0 {
                thread::sleep(Duration::from_millis(200));
                continue;
            }

            self.last_size = self
                .reader
                .stream_position()
                .unwrap_or(self.last_size);

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