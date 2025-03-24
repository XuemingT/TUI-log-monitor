# TUI Log Monitor

A terminal-based system log monitoring dashboard with real-time updates, advanced filtering, and visualization.

## Screenshots

![Log Monitoring View](/screenshots/log_view.png)

## Keyboard Shortcuts and Help

| Shortcut    | Action              |
| ----------- | ------------------- |
| `Tab`       | Switch views        |
| `F`         | Toggle follow mode  |
| `Enter`     | Filter mode         |
| `T`         | Toggle timestamps   |
| `N`         | Toggle line numbers |
| `↑/↓`       | Scroll up/down      |
| `PgUp/PgDn` | Page up/down        |
| `Q`         | Quit                |

## Usage

### Monitor system logs

```bash
sudo cargo run --bin log_monitor /var/log/system.log
```

### Monitor any log file

```bash
cargo run --bin log_monitor /path/to/log/file.log
```
