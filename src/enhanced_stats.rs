use chrono::{DateTime, Local};
use plotters::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingSession {
    pub timestamp: DateTime<Local>,
    pub total_urls: usize,
    pub unique_urls: usize,
    pub files_processed: usize,
    pub processing_time_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnhancedStatistics {
    pub sessions: Vec<ProcessingSession>,
    pub domain_frequencies: HashMap<String, usize>,
}

impl EnhancedStatistics {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            domain_frequencies: HashMap::new(),
        }
    }

    pub fn add_session(&mut self, session: ProcessingSession) {
        self.sessions.push(session);
    }

    pub fn update_domain_frequencies(&mut self, urls: &[String]) {
        for url_str in urls {
            if let Ok(url) = Url::parse(url_str) {
                if let Some(domain) = url.host_str() {
                    // Remove 'www.' prefix if present
                    let clean_domain = domain.strip_prefix("www.").unwrap_or(domain).to_string();
                    *self.domain_frequencies.entry(clean_domain).or_insert(0) += 1;
                }
            }
        }
    }

    pub fn generate_domain_distribution_chart(&self, output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let root = BitMapBackend::new(output_path.to_str().unwrap(), (1600, 900)).into_drawing_area();
        root.fill(&WHITE)?;

        let mut sorted_domains: Vec<_> = self.domain_frequencies.iter().collect();
        sorted_domains.sort_by(|a, b| b.1.cmp(a.1));
        let top_domains: Vec<_> = sorted_domains.into_iter().take(10).collect();

        if top_domains.is_empty() {
            return Ok(());
        }

        let max_freq = top_domains.iter().map(|(_, count)| **count).max().unwrap_or(0) as f64;
        let max_domain_len = top_domains.iter().map(|(domain, _)| domain.len()).max().unwrap_or(0);
        
        // Calculate margins based on domain length
        let bottom_margin = max_domain_len as u32 * 7; // Increase bottom margin for domain names
        
        let mut chart = ChartBuilder::on(&root)
            .caption("Top 10 Domains", ("sans-serif", 30))
            .margin_top(10)
            .margin_right(40)
            .margin_left(60)
            .margin_bottom(bottom_margin) // Use calculated bottom margin
            .x_label_area_size(150) // Increased space for domain labels
            .y_label_area_size(60)
            .build_cartesian_2d(
                0f64..10f64,
                0f64..max_freq * 1.1,
            )?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .bold_line_style(&WHITE.mix(0.3))
            .y_desc("Frequency")
            .x_desc("Domain")
            .x_labels(0)  // Remove default x-axis labels
            .axis_desc_style(("sans-serif", 15))
            .draw()?;

        // Calculate bar width to leave space between bars
        let bar_width = 0.6;  // Make bars slightly narrower
        let bar_margin = (1.0 - bar_width) / 2.0;

        // Draw bars with margins
        for (i, (domain, &count)) in top_domains.iter().enumerate() {
            let x_start = i as f64 + bar_margin;
            let x_end = (i as f64) + bar_width + bar_margin;
            
            // Draw the bar
            chart.draw_series(std::iter::once(Rectangle::new(
                [(x_start, 0.0), (x_end, count as f64)],
                BLUE.filled(),
            )))?;

            // Draw domain label centered under the bar
            let label_x = i as f64 + 0.5;
            
            // Create rotated text style
            let style = TextStyle::from(("sans-serif", 14))
                .transform(FontTransform::Rotate270)
                .color(&BLACK);
            
            // Position the label below the x-axis with more space
            chart.draw_series(std::iter::once(Text::new(
                domain.to_string(),
                (label_x, -max_freq * 0.02), // Reduced negative offset to move labels up
                style,
            )))?;
        }

        root.present()?;
        Ok(())
    }

    pub fn generate_historical_trend_chart(&self, output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let root = BitMapBackend::new(output_path.to_str().unwrap(), (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;

        if self.sessions.is_empty() {
            return Ok(());
        }

        let min_time = self.sessions.first().unwrap().timestamp;
        let max_time = self.sessions.last().unwrap().timestamp;
        let max_urls = self.sessions.iter().map(|s| s.total_urls).max().unwrap_or(0);

        let mut chart = ChartBuilder::on(&root)
            .caption("Historical Processing Trends", ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(50)
            .y_label_area_size(60)
            .build_cartesian_2d(min_time..max_time, 0..max_urls + (max_urls / 10))?;

        chart
            .configure_mesh()
            .x_labels(5)
            .y_labels(10)
            .y_desc("Number of URLs")
            .x_desc("Time")
            .axis_desc_style(("sans-serif", 15))
            .draw()?;

        chart.draw_series(LineSeries::new(
            self.sessions.iter().map(|s| (s.timestamp, s.total_urls)),
            &BLUE,
        ))?;

        root.present()?;
        Ok(())
    }

    pub fn export_report(&self, output_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut report = String::new();
        report.push_str("# URL Processing Statistics Report\n\n");
        
        // Overall statistics
        report.push_str("## Overall Statistics\n");
        report.push_str(&format!("Total Processing Sessions: {}\n", self.sessions.len()));
        
        if let Some(last_session) = self.sessions.last() {
            report.push_str(&format!("Last Processing Time: {}\n", last_session.timestamp));
            report.push_str(&format!("Last Session URLs Processed: {}\n", last_session.total_urls));
            report.push_str(&format!("Last Session Unique URLs: {}\n", last_session.unique_urls));
            report.push_str(&format!("Last Session Files Processed: {}\n", last_session.files_processed));
            report.push_str(&format!("Last Session Processing Time: {:.2}s\n", last_session.processing_time_secs));
        }

        // Domain statistics
        report.push_str("\n## Domain Statistics\n");
        let mut domains: Vec<_> = self.domain_frequencies.iter().collect();
        domains.sort_by(|a, b| b.1.cmp(a.1));
        
        for (domain, count) in domains.iter().take(20) {
            report.push_str(&format!("- {}: {} URLs\n", domain, count));
        }

        // Session history
        report.push_str("\n## Processing History\n");
        for session in self.sessions.iter().rev().take(10) {
            report.push_str(&format!("\nSession at {}:\n", session.timestamp));
            report.push_str(&format!("- Total URLs: {}\n", session.total_urls));
            report.push_str(&format!("- Unique URLs: {}\n", session.unique_urls));
            report.push_str(&format!("- Files Processed: {}\n", session.files_processed));
            report.push_str(&format!("- Processing Time: {:.2}s\n", session.processing_time_secs));
        }

        std::fs::write(output_path, report)?;
        Ok(())
    }
}
