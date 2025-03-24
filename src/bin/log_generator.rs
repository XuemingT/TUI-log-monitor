use std::fs::OpenOptions;
use std::io::Write;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rand::Rng;

fn main() {
    // Add rand to your Cargo.toml: rand = "0.8.5"
    let log_path = "test_application.log";
    let log_levels = ["INFO", "DEBUG", "WARNING", "ERROR"];
    let messages = [
        "Processing user request",
        "Database query completed in 150ms",
        "Cache miss detected for key 'user_profile'",
        "Connection attempt failed: timeout",
        "Authentication successful for user 'admin'",
        "Data validation error: missing required field",
        "Background task started: report generation",
        "Memory usage optimized: freed 250MB",
        "Request received from 192.168.1.1",
        "File not found: config.json",
        "API rate limit reached for client ID #1234",
        "Successfully processed batch job #89754"
    ];
    
    println!("Generating log entries to: {}", log_path);
    println!("Press Ctrl+C to stop");
    
    let mut rng = rand::thread_rng();
    let mut sequence = 1000;
    
    loop {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open log file");
            
        let now = chrono::Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            
        // Choose log level with weighted probability (more INFO than ERROR)
        let level_idx = match rng.gen_range(0..10) {
            0..=6 => 0, // 70% INFO
            7..=8 => 1, // 20% DEBUG
            9 => if rng.gen_bool(0.7) { 2 } else { 3 }, // 7% WARNING, 3% ERROR
            _ => unreachable!()
        };
        
        let level = log_levels[level_idx];
        let message = messages[rng.gen_range(0..messages.len())];
        let log_entry = format!("[{} - {} - #{}] {}\n", 
                               timestamp, 
                               level, 
                               sequence,
                               message);
        
        file.write_all(log_entry.as_bytes()).expect("Failed to write to log");
        sequence += 1;
        
        // Random delay between 0.5-3 seconds
        sleep(Duration::from_millis(rng.gen_range(500..3000)));
    }
}
