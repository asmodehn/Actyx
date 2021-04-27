---
title: ax settings get
---

```text title="Get settings from an ActyxOS node"
USAGE:
    ax settings get [FLAGS] <SCOPE> <NODE>

FLAGS:
    -h, --help           Prints help information
    -l, --local          Process over local network
        --no-defaults    Only return settings explicitly set by the user and
                         skip default values
    -V, --version        Prints version information
    -v                   Verbosity level. Add more v for higher verbosity
                         (-v, -vv, -vvv, etc.)

ARGS:
    <SCOPE>    Scope from which you want to get the settings
    <NODE>     Node ID or, if using `--local`, the IP address of the node to
               perform the operation on
```

:::info Null or schema conformant
The return value of this command will always be either null or schema conformant, unless the `--no-defaults` flag is used.
:::

Here are some examples of using the `ax settings get` command:

```text title="Example Usage"
# Get the settings for the node with settings scope com.actyx at 10.2.3.23
ax settings get --local com.actyx 10.2.3.23

# Just get the displayName setting
ax settings get --local com.actyx/admin/displayName 10.2.3.23
```

import { NPS } from '../../../../src/components/NPS'

<NPS />