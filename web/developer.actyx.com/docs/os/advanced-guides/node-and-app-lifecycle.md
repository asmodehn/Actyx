---
title: Node and App lifecycle
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

As you interact with an ActyxOS node or app, it transitions through different states in its lifecycle. This page explains the lifecycles of ActyxOS nodes and apps, as well as the behaviour you can expect in the different stages. The first section describes the states of nodes and apps, and the second section explains state transitions and the behaviour you can expect.

## States

The states of nodes and apps are a combination of three and four boolean variables, respectively. Note that not all theoretically possible combinations of these variables can actually be achieved, this will become clear in the second section of this page. The state of your node or app can depend on the following variables:

- [Status](#status)
- [Mode](#mode)
- [Settings](#settings)
- [License](#license)

The tables in the following sub-sections will explain the meaning of each variable and its values for ActyxOS nodes and apps.

### Status

The variable _Status_ describes if ActyxOS or an app is running on your node:

Value     | Nodes                               | Apps                            |
----------|-------------------------------------|---------------------------------|
 `Running`| ActyxOS is running on your node     | App is running in its runtime   |
 `Stopped`| ActyxOS is not running on your node | App is not running              |

Note that closing the ActyxOS window on Android will not actually stop ActyxOS – it will keep running in the background. Please have a look at the section [starting and stopping ActyxOS](/docs/os/advanced-guides/actyxos-on-android#starting-and-stopping-actyxos) in the advanced guide for ActyxOS on Android.

### Mode (only apps)

The variable _Mode_ is only available for apps. While the variable _Status_ (see above) describes the actual state of an app, _Mode_ describes the target state from a user's point of view. This might, for example, lead to an app being started on your device if it is in the state `Enabled` and `Stopped`.

Value      | Apps                    | 
-----------|-------------------------|
 `Enabled` | App should be running |
 `Disabled`| App should be stopped |

:::tip Use fleet management services to control this behaviour for your nodes 
ActyxOS does not control the Host system of your nodes, therefore this is only available for apps. You can use fleet management services such as [Balena for ActyxOS on Docker](using-balena) or [Workspace one for ActyxOS on Android](using-workspace-one) to control e.g. restarts of ActyxOS.
:::

### Settings

Nodes and apps can have valid or invalid settings. The validity of your settings depends on whether the settings object that you configured complies with the underlying settings schema:

Value     | Nodes                               | Apps                            |
----------|-------------------------------------|---------------------------------|
 `Valid`  | The settings object under `com.actyx.os` (= your node settings) is valid | The settings object under `<APP ID>` is valid |
 `Invalid`| The settings object under `com.actyx.os` (= your node settings) is invalid      | The settings object under `<APP ID>` is invalid |

If you want to know more about node and app settings and in ActyxOS, please refer to our advanced guide on [Node and App Settings](node-and-app-settings).

### Licenses

This state displays whether the license key you entered for your ActyxOS node or app is valid:

Value     | Nodes                       | Apps                       |
----------|-----------------------------|----------------------------|
 `Valid`  | Node license key is valid   | App license key is valid   |
 `Invalid`| Node license key is invalid | App license key is invalid |

 ## Events and state transitions

 Events define when a node or an app changes one or more of its states. A simple example would be that starting the ActyxOS app on your Android device (if ActyxOS was `Stopped` before) leads to your node transitioning from `Stopped` to `Running`. This state transition would then be caused by the `NodeStarted` event. As you can see in the next section, not all events necessarily trigger always state transitions.

We refer to each combination of node or app states as **lifecycle stages**. Referring to the below illustration of the node lifecycle, a node in the lifecycle stage **Operational** has the states **Running**, **LicenseValid** and **SettingsValid**.

### Node lifecycle
The following illustration shows the lifecycle stages, states and events of an ActyxOS node. Below the illustration you can find an explanation of the different node states.

![Node Lifecycle](/images/os/node-lifecycle.png)

<Tabs
  defaultValue="installed"
  values={[
    { label: 'Installed', value: 'installed', },
    { label: 'Misconfigured', value: 'misconfigured', },
    { label: 'Operational', value: 'operational', },
    { label: 'Configured', value: 'configured', },
  ]
}>
<TabItem value="installed">

A node starts its lifecycle once it is installed on its host system. At that point its states are **Stopped**, **LicenseInvalid**, **SettingsInvalid**. 

From this point only one event leading to a transition is possible:

- **NodeStarted**: leads to the next lifecycle stage: **Misconfigured**.

</TabItem>
<TabItem value="misconfigured">

Once ActyxOS has been started, the nodes states are **Running**, **LicenseInvalid**, **SettingsInvalid**. At this point the node cannot be fully functional, as it does not have valid settings.

In this lifecycle stage, three events can lead to state transitions:
- **NodeStopped**: leads to the lifecycle stage **Installed** (e.g. by stopping the ActyxOS docker container)
- **NodeKilled**: leads to the lifecycle stage **Installed** (e.g. the Android host system kills ActyxOS)
- **NodeSettingsValidated**: (by configuring valid node settings) leads to the lifecycle stage **Operational**. As your licenses are part of the node settings, validated settings always entail a valid license.

One event could happen that will not change the nodes lifecycle stage:
- **NodeSettingsInvalidated**: only happens if one or more node settings are [unset](/docs/cli/ax-settings/unset) while the node already has invalid settings.

:::tip
Please note that you can already deploy apps and/or set app settings at this point. You cannot start the apps though, as starting apps requires an operational node.
:::

</TabItem>
<TabItem value="operational">

You now have an operational node. Its states are **Running**, **LicenseValid** and **SettingsValid**. This should be the lifecycle stage that all your running nodes are in as only this state enables you to run apps on your node (more info on app lifecycles below).

In this lifecycle stage, three events can lead to state transitions:
- **NodeStopped**: leads to the lifecycle stage **Installed** (e.g. by stopping the ActyxOS docker container)
- **NodeKilled**: leads to the lifecycle stage **Installed** (e.g. the Android host system kills ActyxOS)
- **NodeSettingsInvalidated**: only happens if one or more node settings are [unset](/docs/cli/ax-settings/unset) or ActyxOS was updated with a non-backwards-compatible node settings schema change. Note that you cannot `ax settings set` invalid settings, as the command automatically validates against the node settings schema.

One event could happen that will not change the nodes lifecycle stage:
- **NodeSettingsValidated**: caused by every successful `ax settings set` command. You could e.g. just change the display name of your node.

::: Interacting with node settings does not work?
If you want to interact with node settings (either via `ax settings set actyx.com.os` or `ax settings unset actyx.com.os`), all apps on your node must be stopped.
:::

</TabItem>
<TabItem value="configured">

A configured node has valid settings and a valid license, but is currently **Stopped**. You cannot interact with ActyxOS in this lifecycle stage, other than starting it again which will trigger a **NodeStarted** event.

</TabItem>
</Tabs>

### App lifecycle

The following illustration shows the lifecycle stages, states and events of an ActyxOS app. Below the illustration you can find an explanation of the different app states.

![App Lifecycle](/images/os/app-lifecycle.png)

<Tabs
  defaultValue="misconfigured"
  values={[
    { label: 'Misconfigured', value: 'misconfigured', },
    { label: 'Configured', value: 'configured', },
    { label: 'Operational', value: 'operational', },
    { label: 'Waiting for restart', value: 'waiting', },
  ]
}>
<TabItem value="misconfigured">

A node starts its lifecycle once it is installed on a node via `ax apps deploy`. At that point its states are **Stopped**, **Disabled**, **LicenseValid**, **SettingsInvalid**. 

From this point only one event leading to a transition is possible:

- **AppSettingsValidated**: leading to the next lifecycle stage: **Configured**.

:::info Why can you not start the app?
In order to be able to start an app, your node must fulfill the following prerequisites:
- Running node
- Valid node settings
- Valid app settings
:::

</TabItem>
<TabItem value="configured">

In this lifecycle stage the app is still **Stopped** and **Disabled**, but has valid settings.

In this lifecycle stage, three events can lead to state transitions:
- **AppStarted**: leads to the lifecycle stage **Operational**. Please note that depending on your host system, there are multiple ways to start an app as indicated in the illustration. As this is an intentional start of the app, the apps mode will also switch from **Disabled** to **Enabled**.
- **AppSettingsInvalidated**: happens if one or more app settings are [unset](/docs/cli/ax-settings/unset) or your app was updated with a non-backwards-compatible app settings schema change. Note that you cannot `ax settings set` invalid settings, as the command automatically validates against the app settings schema.

One event could happen that will not change the nodes lifecycle stage:
- **AppSettingsValidated**: caused by every successful `ax settings set` command.

:::tip Not able to start an app?
Other than valid app settings, your node must also have valid node settings to be able to start an app.
:::

</TabItem>
<TabItem value="operational">

You now have an operational app. Its states are **Running**, **LicenseValid** and **SettingsValid**. This is the lifecycle stage in which all your running apps are. You cannot change settings of running apps, so the only possible state changes are to **Stopped** and **Disabled** . Whether an app transitions to the lifecycle stage **Configured** (meaning it was **Stopped** and **Disabled**) or to **Waiting for restart** (meaning it was only **Stopped**) depends on the event, and in particular on what triggered the event.

:::info Triggering entities of events
An event can be triggered by different entities. In all but the **AppStopped** case, the triggering entity of the event does not change the effect that the event has on the state change of a node or app. Because the triggering entity matters in this lifecycle stage, the description of the events below contains them.
:::

In this lifecycle stage, the following events can lead to state transitions to **Stopped** and **Disabled**:
- **AppStoppedByActyxCLI**: represents an `ax apps stop` command
- **AppStoppedByHostUI**: can be caused if you use the HostUI, e.g. on Android, to close an app
- **AppStoppedByNodeUI**: can be caused if you use the NodeUI (the ActyxOS window on your node) to close an app

Note that the above events reflect an intentional stop of the app.

An unintentional stop of the app, only leading to a state transition to **Stopped**, is caused by the following events:
- **NodeStopped**: if your node is stopped, all apps running on your node will automatically be stopped
- **NodeKilled**: if your node is killed by the host system, all apps running on your node will automatically be stopped
- **AppKilled**: happens if your app is killed by the host system
- **AppStoppedByHost**: happens if your app is stopped by the host system
- **AppStoppedByNode**: happens if your app is stopped by ActyxOS

</TabItem>
<TabItem value="waiting">

:::tip Corresponding node status
Please note, that at this point your node could be both **Running** or **Stopped**. It depends on the event that caused your app to transition into this state.
:::

In this lifecycle stage your app is **Stopped**, but waiting to be started again as its mode is still **Enabled**. Your app will transition back into the lifecycle stage **Operational** if one of the following events happen:
- **NodeStarted**: upon start, your node will also start all apps that are **Enabled**
- **NodeSettingsValidated**: a node cannot start apps if it has invalid node settings, but might nevertheless have apps in the state **Enabled**. In this case, a node starts all **Enabled** apps after its node settings became valid. As you cannot change node settings while apps are running, the only scenario for this is a non-backawards-compatible change of the ActyxOS node settings schema.
- **AppStartedByNode**: if an **AppStoppedByNode** happened before

</TabItem>
</Tabs>