# watcheRS
Follow lines appended to a file, or append current time to a file at intervals. Written in Rust, makes use of the inode notify (`inotify`) kernel subsystem, or ordinary file descriptor polling.  
Build with `cargo build --all --all-targets --profile release`.
```
$ watcheRS --help
Read the last line, follow and print out any changes in the specified file; or append current time to the file at intervals
[default COMMAND: r(ead)]

Usage: watcheRS <FILE> [COMMAND]

Arguments:
  <FILE>  Path to the file

Options:
  -h, --help     Print help
  -V, --version  Print version

Commands:
  read   [alias: r] Print out the last line in the file, and then follow any incoming changes
    Usage: watcheRS <FILE> read [OPTIONS]
    Options:
      -s, --sleep <SLEEP>        second interval [default: 20]
      -u, --use-polling          Disable inotify and employ polling instead
      
  write  [alias: w] Append current time to the file at specified intervals
    Usage: watcheRS <FILE> write [OPTIONS]
    Options:
      -i, --interval <INTERVAL>  millisecond interval [default: 2000]
      
  help              Print this message or the help of the given subcommand(s)
```
