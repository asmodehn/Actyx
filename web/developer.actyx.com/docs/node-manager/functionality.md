---
title: Functionality
---

The main elements of the applications are the status bar and 5 tabs, which will be explained in more detail in the following.

<img src="/images/node-manager/node-manager-header.png" style={{maxWidth: "550px", marginBottom: "1rem" }} />

### Status Bar

The status bar lets you connect to your nodes by entering the IP address of the node you want to connect to into the text field and hitting the _Connect_ button.
In addition, the bar displays the current connection status of your node at all times on the left.

### Status Tab

The _Status Tab_ displays general information about the node you are currently connected to:

<img src="/images/node-manager/node-manager-status.png" style={{maxWidth: "550px" }} />

- **Connection state**: `unreachable` or `reachable`
- **Node ID**: `<IP address>` or `localhost`
- **Display name**: The display name you defined in the node settings
- **State**: `Stopped` or `Running`
- **Settings**: `Invalid` or `Valid`
- **License**: `Invalid` or `Valid`
- **Apps deployed**: Number of apps that are currently deployed to the node
- **Apps running**: Number of apps that are currently running on the node
- **Started**: Date and time when ActyxOS was started on the node
- **Version**: Version number of ActyxOS running on the node

:::info Node and app lifecycle
For more information on the node's general state or settings and license states, please refer to the [node and app lifecycle documentation](../os/advanced-guides/node-and-app-lifecycle).
:::

Additionally, the _Status Tab_ also displays the logs emitted by the ActyxOS node.

### Apps Tab

The _Apps Tab_ gives an overview of all applications that are installed on the node at a glance:

<img src="/images/node-manager/node-manager-apps.png" style={{maxWidth: "550px" }} />

- **App ID**: The ID you specify in the apps `manifest.yml`. This is also the settings scope of this application
- **Version**: Version number of the application
- **Enabled**: Status of the application (_Disabled_ or _Enabled_)
- **State**: `Stopped` or `Running`
- **Settings**: `Invalid` or `Valid`
- **License**: `Invalid` or `Valid`
- **Started**: Date and time when the application was started
- **Actions**: Options to start, stop and undeploy the application

:::info Node and app lifecycle
For more information on the node's general state or settings and license states, please refer to the [node and app lifecycle documentation](../os/advanced-guides/node-and-app-lifecycle/).
:::

Moreover, the _Apps Tab_ offers the capability to validate and package an application and deploy applications to nodes.
In case you want to validate or package an application, please enter the path to the app directory into the text field.
If you want to deploy an application, please enter the path to the packaged tar.gz file into the text field.

### Settings Tab

The _Settings Tab_ displays all [settings scopes](../os/advanced-guides/node-and-app-settings/#configuring-nodes) that are deployed to the node and their respective settings in an interactive code editor.
The scope of the node is `com.actyx.os` and the scope of the apps are their respective app ID.

<img src="/images/node-manager/node-manager-settings.png" style={{maxWidth: "550px" }} />

You can simply edit the JSON file in the editor to change the settings for your node or for an app.
Every time you edit the settings, your changes will be validated against the JSON schema and can only be saved when settings comply with the schema.
You can view the settings schema by ticking the checkbox in the bottom right corner.

### Tools Tab

The _Tools Tab_ lets you generate a new swarm key and lets you copy it to the clipboard.
A [swarm](../os/guides/swarms/#whats-a-swarm) is defined by a single property, the so-called swarm key.
In order to participate in a swarm, a node must have the secret swarm key.
The swarm key is a setting that must be set for a node to function correctly.

### About Tab

The _About Tab_ displays the Actyx CLI version that the node manager is based on.
Additionally you can see the Software License Agreement and links to our support channels.