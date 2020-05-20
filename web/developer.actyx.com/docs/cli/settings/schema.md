---
title: ax settings schema
---

## Get setting schemas from a node

```bash
$ ax settings schema --help
USAGE: ax settings schema [FLAGS] <SCOPE> <NODE>

FLAGS:
    -v, -vv, -vvv    Increase verbosity
    -h, --help       Prints help information
    --local          Process over local network

ARGS:
    <SCOPE>          Scope at which you want to get the settings.
    <NODE>           Node ID or, if using `--local`, the IP address, of the node
                     to perform the operation on. You may also pass in a file with
                     a value on the first line using the syntax `@file.txt` or have
                     the command read one value per line from stdin using `@-`.
```

Here is a simple example of using the `ax settings schema` command:

```bash
# Get the ActyxOS nodes settings schema from a node
$ ax settings schema --local com.actyx.os 10.2.3.23

# Get the settings schema for a specific app from a node
$ ax settings schema --local com.example.app 10.2.3.23
```