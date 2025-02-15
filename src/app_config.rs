use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub directory: String,
    pub output: String,
    pub skip_header: bool,
    pub workers: usize,
    pub exclude_file: String,
    pub continue_on_error: bool,
    pub timeout: u64,
    pub master_list_path: String,
    pub sample_file_path: String,
    pub selected_header: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            if let Ok(contents) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(config_path, json)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("csv-link-extractor");
        fs::create_dir_all(&path).unwrap_or_default();
        path.push("config.json");
        path
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            directory: String::from("C:\\Users\\AJ\\Downloads\\linkedin-jobs"),
            output: String::from("C:\\Users\\AJ\\Downloads\\all_links.txt"),
            skip_header: false,
            workers: 4,
            exclude_file: String::new(),
            continue_on_error: false,
            timeout: 90,
            master_list_path: String::new(),
            sample_file_path: String::new(),
            selected_header: String::from("Company Apply Url"),
        }
    }
}
