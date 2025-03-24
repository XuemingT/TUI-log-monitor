# Log Monitor TUI

A terminal-based log monitoring tool built to practice TUI development concepts in preparation for the Crankshaft monitoring dashboard project.

## Project Purpose

This application was created as a learning project to explore terminal user interface concepts, particularly:
- Real-time data monitoring and display
- User interaction with keyboard shortcuts
- Structured display of log information
- Statistics and data visualization in the terminal

## Features

- Multi-view interface (Logs, Statistics, Help)
- Real-time log monitoring with auto-follow
- Filtering system for finding specific log entries
- Color-coded log levels
- Customizable display options (timestamps, line numbers)
- Statistics view with log level distribution

## Usage

```bash
cargo run --bin log_monitor /path/to/logfile.log
