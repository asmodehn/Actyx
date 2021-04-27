---
title: ax apps start
---

<!-- TODO NKI: replace with correct link -->

:::warning `ax apps` subcommand is deprecated
Managing apps via the Actyx CLI is deprecated and will no longer be supported in future ActyxOS versions.
:::

```text title="Start an app on an ActyxOS node"
USAGE:
    ax apps start [FLAGS] <APP> <NODE>

FLAGS:
    -h, --help       Prints help information
    -l, --local      Process over local network
    -V, --version    Prints version information
    -v               Verbosity level. Add more v for higher verbosity
                     (-v, -vv, -vvv, etc.)

ARGS:
    <APP>     App ID of the app to start on the given node
    <NODE>    Node ID or, if using `--local`, the IP address of the node to
              perform the operation on
```

Here is an example of using the `ax apps start` command:

```text title="Example Usage"
# Start app with ID com.example.app on node at 10.2.3.23
ax apps start --local com.example.app 10.2.3.23
```

import { NPS } from '../../../../src/components/NPS'

<NPS />