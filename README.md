# TUI Log Monitor

A terminal-based system log monitoring dashboard with real-time updates, advanced filtering, and visualization.

## screenshots

![Log Monitoring View](/screenshots/log_view.png)

# keyboard shortcuts and help

KeyActionTabSwitch viewsFToggle follow mode/Enter filter modeTToggle timestampsNToggle line numbers↑/↓Scroll up/downPgUp/PgDnPage up/downQQuit

# usage

# Monitor system logs

sudo cargo run --bin log_monitor /var/log/system.log

# Monitor any log file

cargo run --bin log_monitor /path/to/log/file.log
