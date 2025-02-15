use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

pub struct MasterList {
    urls: HashSet<String>,
    file_path: Option<String>,
}

impl MasterList {
    pub fn new() -> Self {
        Self {
            urls: HashSet::new(),
            file_path: None,
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let file = File::open(&path)?;
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            if let Ok(url) = line {
                self.urls.insert(url.trim().to_string());
            }
        }
        self.file_path = Some(path.as_ref().to_string_lossy().into_owned());
        Ok(())
    }

    pub fn save(&self) -> io::Result<()> {
        if let Some(path) = &self.file_path {
            let mut file = File::create(path)?;
            for url in &self.urls {
                writeln!(file, "{}", url)?;
            }
        }
        Ok(())
    }

    pub fn contains(&self, url: &str) -> bool {
        self.urls.contains(url)
    }

    pub fn add(&mut self, url: String) {
        self.urls.insert(url);
    }

    pub fn is_loaded(&self) -> bool {
        self.file_path.is_some()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.urls.clear();
        self.file_path = None;
    }
}
