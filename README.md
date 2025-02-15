# CSV Link Extractor

A GUI application that processes CSV files to extract and manage URLs, with special handling for LinkedIn job links.

## Features

- GUI interface for easy operation
- Process multiple CSV files in a directory
- Configurable settings that persist between sessions:
  - Input directory path
  - Output file location
  - Column header selection
  - Error handling preferences
- URL Processing:
  - Extracts URLs from a specified CSV column
  - Automatically converts LinkedIn job-apply links to view links
  - Validates URLs using regex pattern matching
  - Deduplicates URLs across all processed files
- Master List Management:
  - Maintains a master list of previously processed URLs
  - Only outputs new, unique URLs not in the master list
  - Automatically updates master list with new URLs
- Additional Features:
  - Optional URL exclusion list support
  - Configurable error handling (continue or stop on errors)
  - Sample CSV file header detection

## Usage

1. Launch the application
2. Configure settings:
   - Set the directory containing CSV files
   - Choose output file location
   - Select URL column from detected headers
   - Configure master list file (optional)
   - Set additional options as needed
3. Click "Process" to start URL extraction

## Building

```bash
cargo build --release
```

The compiled application will be available in `target/release/export_csv_links.exe`

## Requirements

- Windows operating system
- CSV files with consistent column headers
- URLs must be in standard HTTP/HTTPS format
