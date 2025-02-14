use clap::Parser;
use csv::StringRecord;
use once_cell::sync::Lazy;
use rayon::ThreadPoolBuilder;
use regex::Regex;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use eframe::{egui, App, Frame, NativeOptions};
use egui::{CentralPanel, TextEdit, Layout, Direction, Button, ViewportBuilder, Style, Visuals, FontDefinitions, FontFamily, FontId};

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

fn extract_urls_from_csv(csv_filepath: &PathBuf, skip_header: bool, continue_on_error: bool) -> Vec<String> {
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

    let url_index = match headers.iter().position(|h| h == "Company Apply Url") {
        Some(i) => i,
        None => {
            eprintln!(
                "Error: 'Company Apply Url' column not found in file {:?}",
                csv_filepath
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
) {
    let urls = extract_urls_from_csv(&csv_filepath, skip_header, continue_on_error);
    let mut set = dedup_urls.lock().unwrap();
    for url in urls {
        set.insert(url);
    }
}

fn process_directory(
    directory_path: PathBuf,
    output_filepath: PathBuf,
    workers: usize,
    skip_header: bool,
    exclude_file: Option<PathBuf>,
    continue_on_error: bool,
) {
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
            s.spawn(move |_| {
                process_file(file, dedup_urls, skip_header, continue_on_error);
            });
        }
    });

    let file = match File::create(&output_filepath) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error creating output file {:?}: {}", output_filepath, e);
            return;
        }
    };
    let mut writer = BufWriter::new(file);

    let set = dedup_urls.lock().unwrap();
    for url in set.iter() {
        if !excluded_urls.contains(url) {
            if let Err(e) = writeln!(writer, "{}", url) {
                eprintln!("Error writing to output file: {}", e);
            }
        }
    }
    println!(
        "URLs from all CSV files in {:?} extracted, deduplicated, and saved to {:?}",
        directory_path, output_filepath
    );
}

struct ExportCsvLinksApp {
    directory: String,
    output: String,
    skip_header: bool,
    workers: usize,
    exclude_file: String,
    continue_on_error: bool,
    timeout: u64,
}

impl Default for ExportCsvLinksApp {
    fn default() -> Self {
        Self {
            directory: String::from("C:\\Users\\AJ\\Downloads\\linkedin-jobs"),
            output: String::from("C:\\Users\\AJ\\Downloads\\all_links.txt"),
            skip_header: false,
            workers: 4,
            exclude_file: String::new(),
            continue_on_error: false,
            timeout: 90,
        }
    }
}

impl App for ExportCsvLinksApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let mut style = (*ctx.style()).clone(); // Get the current style

        // Modify colors
        style.visuals.dark_mode = true;
        style.visuals.override_text_color = Some(egui::Color32::WHITE);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(30, 30, 30);
        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(50, 50, 50);

        // Modify spacing
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.window_margin = egui::Margin::same(10.0);

        // Modify shapes
        style.visuals.window_rounding = egui::Rounding::same(5.0);

        ctx.set_style(style);

        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Export CSV Links");

            ui.horizontal(|ui| {
                ui.label("Directory:");
                ui.add(TextEdit::singleline(&mut self.directory));
            });

            ui.horizontal(|ui| {
                ui.label("Output File:");
                ui.add(TextEdit::singleline(&mut self.output));
            });

            ui.checkbox(&mut self.skip_header, "Skip Header");

            ui.horizontal(|ui| {
                ui.label("Workers:");
                ui.add(egui::Slider::new(&mut self.workers, 1..=16).integer());
            });

             ui.horizontal(|ui| {
                ui.label("Exclude File:");
                ui.add(TextEdit::singleline(&mut self.exclude_file));
            });

            ui.checkbox(&mut self.continue_on_error, "Continue on Error");

            ui.horizontal(|ui| {
                ui.label("Timeout:");
                ui.add(egui::Slider::new(&mut self.timeout, 1..=300).integer());
            });


            if ui.button("Process").clicked() {
                let directory_path = PathBuf::from(self.directory.clone());
                let output_path = PathBuf::from(self.output.clone());
                let exclude_file_path = if !self.exclude_file.is_empty() {
                    Some(PathBuf::from(self.exclude_file.clone()))
                } else {
                    None
                };

                process_directory(
                    directory_path,
                    output_path,
                    self.workers,
                    self.skip_header,
                    exclude_file_path,
                    self.continue_on_error,
                );
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(640.0, 480.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Export CSV Links",
        options,
        Box::new(|_cc| Box::new(ExportCsvLinksApp::default())),
    )
}
