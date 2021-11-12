# scanwatch

## summary

simple utility to watch a directory and forward any new pdf file
within to a printing frontend

This software is developed for my own personal use. However, if you
have suggestions, feel free to ask me.

**LINUX ONLY**

## workflow

- look for configuration file 'scanwatch.toml', either from the current directory or from the XDG home configuration directory (usually `~/.config/scanwatch/scanwatch.toml`)
- read action rules from config file
- watch path for new file creation using inotify
- display message using desktop notification
- execute the command that is given by the matching rule, e.g. `lpr -P{printer-name} {document-name}` for every file that matches the above rule

## example configurations

    path = "~/scanwatch"
    
    [rules.xpdf]
    filter = "\\.pdf$"
    cmd = "xpdf"
    args = ["{filename}"]
    msg = "opening '{filename:short}' with xpdf"
    x = ""

    [rules.lpr]
    filter = "^CN.*\\.pdf$"
    path = "test-lpr"
    cmd = "lpr"
    args = ["-P{x}", "{filename}"]
    msg = "sending '{filename}' to printer '{x}'"
    x = "kyo_einseitig"

## disclaimer

Of course, it would have been **much** easier to write this using a
shell script. But rust is so great, I wanted to play with it and maybe
I will enhance this a little more later on...

## status

**[05.11.2021]**: The given directory is being watched and if a new file is created, a command can be executed. UNFORTUNATELY, the old printer frontend gtklp is no longer properly maintained, at least it does not work on my
ArchLinux system. Therefore this is still not functional.

**[06.11.2021]**: There is a certain flaw with this approach: notify
(or inotify) under Linux does not watch any notification changes from
files on smb network shares. This was initially my use case. Since
there seems to be no way around (other than manually polling the
directory's content all the time), I changed the use case a little:

Scanner scans file, converts it to pdf and stores it on a network
share that is located on the computer that is running
`scanwatch`. Since it is a local directory, scanwatch is able to watch
the share's directory and can execute the lpr command for every newly
created or overwritten file.

**[07.11.2021]**: Changed from notify to inotify, allow configuration
file in XDG configuration directory, allow multiple rules

**[12.11.2021]**: configuration file has been completely reworked:
only single watch path (global),for every rule we can have a filter
expression (rust syntax) and a certain symbol
