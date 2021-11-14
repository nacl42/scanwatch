# scanwatch

## summary

simple utility to watch a directory and forward any new pdf file
within to a printing frontend (**linux only!**)

This software is developed for my own personal use. However, if you
have suggestions, feel free to contact me.

## use cases

- watch scanner directory for new files and forward them to the printer
- watch scanner directory for new files and open them in pdf viewer
- watch download directory for new files with a certain name and move
  them to a certain folder

## features

- execute multiple actions when a new file is created
- display notification upon action (with custom icon)
- path can include '~' for user home directory

## limitations

- Linux only
- It is not possible to watch SMB mounted paths (i.e. Windows Shares),
  because the underlying linux notification library (inotify) does not
  support it.
- Only one path (and its subdirectories) can be watched. Originally I
  had intended to specify the path for each rule, but the current
  implementation was much easier.


## configuration

Example configuration file:

    # scanwatch.toml
    path = "~/scanwatch"
    
    [rules.xpdf]
    filter = "\\.pdf$"
    cmd = "xpdf"
    args = ["{filename}"]
    msg = "opening '{filename:short}' with xpdf"
    x = ""

    [rules.lpr]
    filter = "^CN.*\\.pdf$" # scanner files on my printer are named CNxxxx.pdf
    cmd = "lpr"
    args = ["-P{x}", "{filename}"]
    msg = "sending '{filename}' to printer '{x}'"
    x = "kyo_einseitig"  # this is the printer name

Valid variables:

| scope  | variable | meaning |
| ------ | -------- | ------- |
| global | path     | path to watch (including subdirectories) | 
| rule   | filter   | filter expression (rust regular expression) |
| rule   | cmd      | command to execute if watched file matches rule |
| rule   | args     | arguments to pass to the given command |
| rule   | x, y, z  | optional variables that can be used in the args variable |
| rule   | msg      | message to display in the notification |

Args and msg can contain the following placeholder variables:

| placeholder | expansion to |
| ----------- | ------------ |
| {filename}    | matched filename (complete path) |
| {filename:short} | matched filename (without watched path prefix) |
| {x}, {y}, {z}  | rule variable 'x', 'y' or 'z' |

## internal workflow

- look for configuration file 'scanwatch.toml', either from the
  current directory or from the XDG home configuration directory
  (usually `~/.config/scanwatch/scanwatch.toml`)
- read global watch path from config file
- read action rules from config file
- watch path for new file creation using inotify
- upon new file creation or existing file write:
- display message using desktop notification
- execute the command that is given by the matching rule, e.g. `lpr
  -P{printer-name} {document-name}` for every file that matches the
  above rule
  
## future plans

- add proper meta information, update version number
- add fancy app logo
- optional audio bell
- add command line options (help, verbose, location of config file)
- simplify filter mechanism

## disclaimer

Of course, it would have been **much** easier to write this using a
shell script. But rust is so great, I wanted to play with it and maybe
I will enhance this a little more later on...
