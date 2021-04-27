---
title: ax settings scopes
---

```text title="Get setting scopes from an ActyxOS node"
USAGE:
    ax settings scopes [FLAGS] <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

Here is an example of using the `ax settings scopes` command:

```text title="Example Usage"
# Get all the settings scopes from node at 10.2.3.23
ax settings scopes --local 10.2.3.23
```

import { NPS } from '../../../../src/components/NPS'

<NPS />