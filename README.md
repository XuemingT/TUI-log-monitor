# TUI Log Monitor

A terminal-based system log monitoring dashboard with real-time updates, advanced filtering, and visualization.

## dashboard screenshots

![Log Monitoring View](/screenshots/log_view.png)

## statistics dashboard

Get insights about your logs with interactive gauges showing distribution:

![Statistics View](/screenshots/stats_view.png)

## keyboard shortcuts and help

| Key       | Action              |
| --------- | ------------------- |
| Tab       | Switch views        |
| F         | Toggle follow mode  |
| /         | Enter filter mode   |
| T         | Toggle timestamps   |
| N         | Toggle line numbers |
| ↑/↓       | Scroll up/down      |
| PgUp/PgDn | Page up/down        |
| Q         | Quit                |

## usage

```bash
# Monitor system logs
sudo cargo run --bin log_monitor /var/log/system.log

# Monitor any log file
cargo run --bin log_monitor /path/to/log/file.log
```

## built for exploring TUI development

This project was created to explore terminal user interface concepts in preparation for the Crankshaft monitoring dashboard project. Key learning areas include:

- Real-time data visualization in the terminal
- User interaction patterns for terminal applications
- State management for complex interfaces
- Multi-view application architecture using Ratatui
