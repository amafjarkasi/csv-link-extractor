# CSV Link Extractor

A Rust application to extract, deduplicate, and export URLs from CSV files within a specified directory. It provides a graphical user interface (GUI) built with `egui` for easy configuration and operation.

## Features

*   **Directory Input:** Specify the directory containing the CSV files to process.
*   **Output File:** Define the path for the output text file where the extracted URLs will be saved.
*   **Skip Header:** Option to skip the first data record (not the header row) in CSV files.
*   **Worker Threads:** Configure the number of worker threads for concurrent processing to speed up the extraction process.
*   **Exclude File:** Provide a file containing URLs to exclude from the output (one URL per line).
*   **Continue on Error:** Option to continue processing even if some files produce errors.
*   **Timeout:** Set a timeout for HTTP requests.
*   **GUI:** User-friendly interface built with `egui` for easy configuration.
*   **URL Deduplication:** Automatically removes duplicate URLs from the output.

## Usage

1.  **Clone the repository:**

    ```bash
    git clone <repository_url>
    cd export_csv_links
    ```

2.  **Build the application:**

    ```bash
    cargo build --release
    ```

3.  **Run the application:**

    ```bash
    cargo run --release
    ```

    Alternatively, you can run the executable directly from the `target/release` directory:

    ```bash
    ./target/release/export_csv_links
    ```

4.  **Using the GUI:**

    *   Enter the path to the directory containing the CSV files.
    *   Enter the desired path for the output text file.
    *   Configure the options as needed (skip header, number of workers, exclude file, continue on error, timeout).
    *   Click the "Process" button to start the extraction.

## Command-Line Arguments

The application also supports command-line arguments for non-GUI usage:

*   `-d, --directory <PATH>`: Path to the directory containing CSV files.
*   `-o, --output <PATH>`: Path to the output text file (default: `all_urls.txt`).
*   `-s, --skip-header`: Skip the first record of data in CSV files.
*   `-w, --workers <NUM>`: Number of worker threads (default: 4).
*   `--exclude-file <PATH>`: Path to a file containing URLs to exclude.
*   `--continue-on-error`: Continue processing even if some files produce errors.
*   `-t, --timeout <SECONDS>`: Timeout for HTTP requests in seconds (default: 90).

## Dependencies

*   `clap`: For command-line argument parsing.
*   `csv`: For reading CSV files.
*   `once_cell`: For lazy initialization of the URL regex.
*   `rayon`: For parallel processing.
*   `regex`: For URL validation.
*   `eframe` and `egui`: For the GUI.
