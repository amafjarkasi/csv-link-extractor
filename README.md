# CSV Link Extractor

A GUI application that processes CSV files to extract and manage URLs, with special handling for LinkedIn job links.

## Features

- Modern dark-themed GUI interface with fixed window size (400x660)
- Process multiple CSV files in parallel from a selected directory
- Comprehensive settings persisted between sessions:
  - Input directory path
  - Output file location
  - Column header selection
  - Error handling preferences
  - Worker thread count (1-16)
  - Request timeout configuration

## URL Processing
- Auto-detects CSV headers from sample file
- Extracts URLs from specified column
- Validates URLs using regex pattern matching
- Converts LinkedIn job-apply links to view links
- Deduplicates URLs across all processed files
- Supports URL exclusion list

## Master List Management
- Maintains persistent master list of processed URLs
- Filters out previously processed URLs
- Auto-updates master list with new entries

## Statistics Dashboard
- Real-time processing statistics
- Tracks:
  - Total files processed
  - Total URLs found
  - Unique URLs count
  - Excluded URLs count
  - Duplicate URLs detected
  - Processing time
  - Last run timestamp
- Statistics persist between sessions

## Screenshots

### Main Interface (2024-02-08)
![Main Interface](./screenshots/main-interface.png)
*Dark theme interface with URL processing controls*

### Statistics Dashboard (2024-02-08)
![Statistics Dashboard](./screenshots/statistics-dashboard.png)
*Real-time processing statistics and history*

## Usage

1. Launch the application
2. Configure settings:
   - Select directory containing CSV files
   - Choose output file location
   - Load a sample CSV to detect headers
   - Select URL column from detected headers
   - Set master list file (optional)
   - Configure exclusion list (optional)
   - Adjust worker threads and timeout
3. Click "Process" to start extraction
4. View results in Statistics tab

## Building

```bash
cargo build --release
```

The compiled application will be available in `target/release/export_csv_links.exe`

## Requirements

- Windows operating system
- CSV files with consistent column headers
- URLs must be in standard HTTP/HTTPS format
