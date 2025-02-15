use clap::Parser;
use csv::StringRecord;
use once_cell::sync::Lazy;
use rayon::ThreadPoolBuilder;
use regex::Regex;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use eframe::{egui, App, Frame, NativeOptions, Storage};
use egui::{CentralPanel, TextEdit, TopBottomPanel};
use chrono::Local;
mod master_list;
use master_list::MasterList;
mod app_config;
use app_config::{AppConfig, Statistics};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the directory containing CSV files
    directory: PathBuf,

    /// Path to the output text file (default: all_urls.txt)
    #[arg(short, long, default_value = "all_urls.txt")]
    output: PathBuf,

    /// Skip the first record of data (not the header row) in CSV files
    #[arg(short, long)]
    skip_header: bool,

    /// Number of worker threads for concurrent processing (default: 4)
    #[arg(short, long, default_value_t = 4)]
    workers: usize,

    /// Path to a file containing URLs to exclude (one URL per line)
    #[arg(long)]
    exclude_file: Option<PathBuf>,

    /// Continue processing even if some files produce errors
    #[arg(long, default_value_t = false)]
    continue_on_error: bool,

    /// Timeout for HTTP requests in seconds (default: 90)
    #[arg(short, long, default_value_t = 90)]
    timeout: u64,
}

// Compile the URL validation regex once
static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^http[s]?://(?:[a-zA-Z0-9\$\-_@.&+!*\(\),]|(?:%[0-9a-fA-F]{2}))+"
    )
    .expect("Invalid regex")
});

fn is_valid_url(url: &str) -> bool {
    URL_REGEX.is_match(url)
}

fn extract_urls_from_csv(
    csv_filepath: &PathBuf,
    skip_header: bool,
    continue_on_error: bool,
    header_name: &str,
) -> Vec<String> {
    let mut urls = Vec::new();
    let file = match File::open(csv_filepath) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening CSV file {:?}: {}", csv_filepath, e);
            return urls;
        }
    };

    let mut rdr = csv::Reader::from_reader(file);
    let headers = match rdr.headers() {
        Ok(h) => h.clone(),
        Err(e) => {
            eprintln!("Error reading headers from {:?}: {}", csv_filepath, e);
            if !continue_on_error {
                return urls;
            }
            StringRecord::new()
        }
    };

    let url_index = match headers.iter().position(|h| h == header_name) {
        Some(i) => i,
        None => {
            eprintln!(
                "Error: '{}' column not found in file {:?}",
                header_name, csv_filepath
            );
            return urls;
        }
    };

    let mut records = rdr.records();
    if skip_header {
        records.next();
    }

    for result in records {
        let record: StringRecord = match result {
            Ok(rec) => rec,
            Err(e) => {
                eprintln!("Error reading record in {:?}: {}", csv_filepath, e);
                if !continue_on_error {
                    return urls;
                }
                continue;
            }
        };

        if let Some(url_field) = record.get(url_index) {
            let trimmed = url_field.trim();
            if !trimmed.is_empty() {
                let replaced = trimmed.replace("linkedin.com/job-apply/", "linkedin.com/jobs/view/");
                if is_valid_url(&replaced) {
                    urls.push(replaced);
                }
            }
        }
    }
    urls
}

fn process_file(
    csv_filepath: PathBuf,
    dedup_urls: Arc<Mutex<HashSet<String>>>,
    skip_header: bool,
    continue_on_error: bool,
    header_name: String,
) {
    let urls = extract_urls_from_csv(&csv_filepath, skip_header, continue_on_error, &header_name);
    let mut set = dedup_urls.lock().unwrap();
    for url in urls {
        set.insert(url);
    }
}

fn process_directory(
    directory_path: PathBuf,
    workers: usize,
    skip_header: bool,
    exclude_file: Option<PathBuf>,
    continue_on_error: bool,
    header_name: String,
) -> HashSet<String> {
    let entries = fs::read_dir(&directory_path).unwrap_or_else(|e| {
        panic!("Error reading directory {:?}: {}", directory_path, e);
    });
    let csv_files: Vec<PathBuf> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("csv"))
                .unwrap_or(false)
            {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    let dedup_urls = Arc::new(Mutex::new(HashSet::new()));

    let pool = ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .unwrap();

    let excluded_urls: HashSet<String> = exclude_file
        .map(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|e| {
                    eprintln!("Error reading exclude file: {}", e);
                    String::new()
                })
                .lines()
                .map(|line| line.trim().to_string())
                .collect()
        })
        .unwrap_or_else(HashSet::new);

    pool.scope(|s| {
        for file in csv_files {
            let dedup_urls = Arc::clone(&dedup_urls);
            let header = header_name.clone();
            s.spawn(move |_| {
                process_file(file, dedup_urls, skip_header, continue_on_error, header);
            });
        }
    });

    let set = dedup_urls.lock().unwrap();
    let mut filtered_urls = HashSet::new();
    for url in set.iter() {
        if !excluded_urls.contains(url) {
            filtered_urls.insert(url.clone());
        }
    }
    filtered_urls
}

#[derive(PartialEq)]
enum Tab {
    Main,
    Statistics,
}

struct ExportCsvLinksApp {
    directory: String,
    output: String,
    skip_header: bool,
    workers: usize,
    exclude_file: String,
    continue_on_error: bool,
    timeout: u64,
    master_list: MasterList,
    master_list_path: String,
    sample_file_path: String,
    available_headers: Vec<String>, 
    selected_header: String,
    config: AppConfig,
    status_message: String,
    current_tab: Tab,
    statistics: Statistics,
}

impl Default for ExportCsvLinksApp {
    fn default() -> Self {
        let config = AppConfig::load();
        let mut app = Self {
            directory: config.directory.clone(),
            output: config.output.clone(),
            skip_header: config.skip_header,
            workers: config.workers,
            exclude_file: config.exclude_file.clone(),
            continue_on_error: config.continue_on_error,
            timeout: config.timeout,
            master_list: MasterList::new(),
            master_list_path: config.master_list_path.clone(),
            sample_file_path: config.sample_file_path.clone(),
            available_headers: Vec::new(),
            selected_header: config.selected_header.clone(),
            config: config.clone(), // Clone config before moving
            status_message: String::from("Ready"),
            current_tab: Tab::Main,
            statistics: config.statistics.clone(),
        };
        
        // Load the CSV headers during initialization
        app.load_sample_csv();
        app
    }
}

impl ExportCsvLinksApp {
    fn load_sample_csv(&mut self) {
        if let Ok(file) = File::open(&self.sample_file_path) {
            let mut rdr = csv::Reader::from_reader(file);
            if let Ok(headers) = rdr.headers() {
                self.available_headers = headers
                    .iter()
                    .map(|h| h.to_string())
                    .collect();
                // If current selected header isn't in the list, select first available
                if !self.available_headers.contains(&self.selected_header) {
                    self.selected_header = self.available_headers
                        .first()
                        .map(|h| h.to_string())
                        .unwrap_or_default();
                }
            }
        }
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let mut style = (*ctx.style()).clone(); // Get the current style
        style.visuals.dark_mode = true;
        style.visuals.override_text_color = Some(egui::Color32::WHITE);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(30, 30, 30);
        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(50, 50, 50);
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.window_margin = egui::Margin::same(10.0);
        style.visuals.window_rounding = egui::Rounding::same(5.0);
        ctx.set_style(style);

        TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(self.current_tab == Tab::Main, "Main").clicked() {
                    self.current_tab = Tab::Main;
                }
                if ui.selectable_label(self.current_tab == Tab::Statistics, "Statistics").clicked() {
                    self.current_tab = Tab::Statistics;
                }
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.current_tab {
                    Tab::Main => self.render_main_tab(ui),
                    Tab::Statistics => self.render_statistics_tab(ui),
                }
            });

            // Status bar at the bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(4.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(&self.status_message);
                });
            });
        });

        // Check for any UI changes
        if ctx.input(|i| i.pointer.any_pressed() || i.key_pressed(egui::Key::Enter)) {
            self.save_config();
        }
    }

    fn save_config(&mut self) {
        self.config.directory = self.directory.clone();
        self.config.output = self.output.clone();
        self.config.skip_header = self.skip_header;
        self.config.workers = self.workers;
        self.config.exclude_file = self.exclude_file.clone();
        self.config.continue_on_error = self.continue_on_error;
        self.config.timeout = self.timeout;
        self.config.master_list_path = self.master_list_path.clone();
        self.config.sample_file_path = self.sample_file_path.clone();
        self.config.selected_header = self.selected_header.clone();
        self.config.statistics = self.statistics.clone();

        if let Err(e) = self.config.save() {
            eprintln!("Error saving config: {}", e);
        }
    }

    fn update_statistics(&mut self, 
        files_processed: usize,
        all_urls: &HashSet<String>,
        excluded_urls: &HashSet<String>,
        start_time: std::time::Instant,
        unique_count: usize
    ) {
        self.statistics = Statistics {
            total_files_processed: files_processed,
            total_urls_found: all_urls.len(),
            unique_urls: unique_count,
            excluded_urls: excluded_urls.len(),
            duplicate_urls: all_urls.len() - unique_count - excluded_urls.len(),
            processing_time: start_time.elapsed().as_secs_f64(),
            last_run: Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string()),
        };
        
        // Save statistics to config
        self.config.statistics = self.statistics.clone();
        self.save_config();
    }

    fn render_main_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Export CSV Links");
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label("Directory:");
            ui.add(TextEdit::singleline(&mut self.directory));

            ui.label("Output File:");
            ui.add(TextEdit::singleline(&mut self.output));

            ui.checkbox(&mut self.skip_header, "Skip Header");

            ui.label("Workers:");
            ui.add(egui::Slider::new(&mut self.workers, 1..=16).integer());

            ui.label("Exclude File:");
            ui.add(TextEdit::singleline(&mut self.exclude_file));

            ui.checkbox(&mut self.continue_on_error, "Continue on Error");

            ui.label("Timeout:");
            ui.add(egui::Slider::new(&mut self.timeout, 1..=300).integer());

            ui.label("Master List File:");
            if ui.text_edit_singleline(&mut self.master_list_path).changed() {
                if Path::new(&self.master_list_path).exists() {
                    if let Err(e) = self.master_list.load_from_file(&self.master_list_path) {
                        eprintln!("Error loading master list: {}", e);
                    }
                }
            }

            if self.master_list.is_loaded() {
                ui.label("Master list is loaded and will filter processed URLs");
            }

            // Add sample CSV loader
            ui.label("Sample CSV:");
            if ui.text_edit_singleline(&mut self.sample_file_path).changed() {
                if Path::new(&self.sample_file_path).exists() {
                    self.load_sample_csv();
                }
            }

            // Add column selector
            if !self.available_headers.is_empty() {
                ui.label("URL Column:");
                egui::ComboBox::from_id_source("header_selector")
                    .selected_text(&self.selected_header)
                    .show_ui(ui, |ui| {
                        for header in &self.available_headers {
                            ui.selectable_value(
                                &mut self.selected_header,
                                header.clone(),
                                header
                            );
                        }
                    });
            }

            // Style the Process button with better contrast
            let process_button = egui::Button::new("Process")
                .fill(egui::Color32::from_rgb(28, 113, 216))  // Changed to a vibrant blue
                .stroke(egui::Stroke::none());
                
            if ui.add(process_button).clicked() {
                self.status_message = "Processing...".to_string();
                let start_time = std::time::Instant::now();
                
                let directory_path = PathBuf::from(self.directory.clone());
                
                // Fix the ownership issue in files_processed counting
                let files_processed = fs::read_dir(&directory_path)
                    .map(|entries| entries
                        .filter(|entry| {
                            entry.as_ref()
                                .ok()
                                .map(|e| {
                                    e.path()
                                        .extension()
                                        .and_then(|ext| ext.to_str())
                                        .map(|ext| ext.eq_ignore_ascii_case("csv"))
                                        .unwrap_or(false)
                                })
                                .unwrap_or(false)
                        })
                        .count())
                    .unwrap_or(0);

                let output_path = PathBuf::from(self.output.clone());
                let exclude_file_path = if !self.exclude_file.is_empty() {
                    Some(PathBuf::from(self.exclude_file.clone()))
                } else {
                    None
                };

                let excluded_urls: HashSet<String> = exclude_file_path
                    .as_ref()
                    .map(|path| {
                        fs::read_to_string(path)
                            .unwrap_or_else(|e| {
                                eprintln!("Error reading exclude file: {}", e);
                                String::new()
                            })
                            .lines()
                            .map(|line| line.trim().to_string())
                            .collect()
                    })
                    .unwrap_or_else(HashSet::new);

                // Get the URLs from processing and store in a variable we won't move
                let all_urls_set = process_directory(
                    directory_path.clone(),
                    self.workers,
                    self.skip_header,
                    exclude_file_path,
                    self.continue_on_error,
                    self.selected_header.clone(),
                );

                // Write results to both output file and master list
                if let Ok(file) = File::create(&output_path) {
                    let mut writer = BufWriter::new(file);
                    let mut count = 0;
                    for url in &all_urls_set {  // Use reference to avoid moving
                        if !excluded_urls.contains(url) && !self.master_list.contains(url) {
                            if let Err(e) = writeln!(writer, "{}", url) {
                                self.status_message = format!("Error writing to file: {}", e);
                                break;
                            }
                            self.master_list.add(url.clone());
                            count += 1;
                        }
                    }

                    // Save updated master list
                    if self.master_list.is_loaded() {
                        if let Err(e) = self.master_list.save() {
                            self.status_message = format!("Error saving master list: {}", e);
                        }
                    }

                    self.update_statistics(
                        files_processed,
                        &all_urls_set,  // Pass reference
                        &excluded_urls,
                        start_time,
                        count
                    );

                    self.status_message = format!("Processed {} unique URLs", count);
                } else {
                    self.status_message = "Error creating output file".to_string();
                }
            }
        });
    }

    fn render_statistics_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Statistics Dashboard");
        ui.add_space(10.0);
        egui::Grid::new("stats_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .show(ui, |ui| {
                ui.label("Total Files Processed:");
                ui.label(format!("{}", self.statistics.total_files_processed));
                ui.end_row();

                ui.label("Total URLs Found:");
                ui.label(format!("{}", self.statistics.total_urls_found));
                ui.end_row();

                ui.label("Unique URLs:");
                ui.label(format!("{}", self.statistics.unique_urls));
                ui.end_row();

                ui.label("Excluded URLs:");
                ui.label(format!("{}", self.statistics.excluded_urls));
                ui.end_row();

                ui.label("Duplicate URLs:");
                ui.label(format!("{}", self.statistics.duplicate_urls));
                ui.end_row();

                ui.label("Processing Time:");
                ui.label(format!("{:.2}s", self.statistics.processing_time));
                ui.end_row();

                if let Some(last_run) = &self.statistics.last_run {
                    ui.label("Last Run:");
                    ui.label(last_run);
                    ui.end_row();
                }
            });
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        self.save_config();
    }
}

impl App for ExportCsvLinksApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.update(ctx, _frame);
    }

    fn save(&mut self, _storage: &mut dyn Storage) { // Added underscore to unused parameter
        self.save_config();
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(400.0, 660.0))
            .with_resizable(false), // Disable window resizing
        persist_window: true,
        ..Default::default()
    };
    
    eframe::run_native(
        "Export CSV Links",
        options,
        Box::new(|_cc| Box::new(ExportCsvLinksApp::default())),
    )
}
