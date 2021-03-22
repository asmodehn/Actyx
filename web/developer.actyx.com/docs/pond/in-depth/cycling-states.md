---
title: Designing for States that can Cycle
hide_table_of_contents: true
---

A state that can cycle is modelled by a finite state machine that is not monotonic: It can go back and forth repeatedly between different states.
A simple example is a worker doing an activity: The worker may start and stop working (taking breaks) repeatedly, switching between states `idle` and `working`.

However, when working with Actyx Pond, this conceptionally cyclical state machine must be translated into a collection of monotonic ones:
In the example, every `workStarted` event that puts the state into `working` should kick off a discrete `working`-state with a unique ID,
and `workStopped` events should explicitly designate which `working`-states they terminate, by referring to the IDs.

A simple example of how this looks in practice:

```ts
type WorkStarted = {
  eventType: 'workStarted'
  workerId: string
  activityId: string
}

type WorkStopped = {
  eventType: 'workStopped'
  workerId: string
  activityId: string

  endedWorkIds: string[]
}

type WorkStatusChanged = WorkStarted | WorkStopped

const WorkerTag = Tag('worker')
const ActivityTag = Tag('activity')
const WorkStatusChangedTag = Tag<WorkStatusChanged>('workStatusChanged')

type State = {
  openWorkIds: string[]
}

const onEvent: Reduce<State, WorkStatusChanged>  = (state, event, metadata) => {
  if (event.type === 'workStarted') {
    // Use the event id as unique id for the work
    state.openWorkIds.push(metadata.eventId)
  } else {
    // Remove all work ids that were ended by this event
    state.openWorkIds = state.openWorkIds.filter(x => !event.endedWorkIds.includes(x))
  }
}

const endAllOpenWork = (pond, workerId, activityId) =>
  pond.run(
    mkWorkerActivityFish(workerId, activityId),
    // End all locally known work assignments
    (state, enqueue) => enqueue(
      WorkStatusChangedTag.and(workerTag.withId(workerId)).and(activityTag.withId(activityId)),
      { eventType: 'workStopped', endedWorkIds: state.openWorkIds }
    )
  )
```

______

So why this complication? Actyx uses a special algorithm to sort events: [Lamport clock](https://en.wikipedia.org/wiki/Lamport_timestamp).
The advantage of a Lamport clock is that it is guaranteed to capture causality even when device clocks are wrong:
E.g. a given `workStopped` event will always come after the `workStarted` events it refers to, even if the device clock on the node is accidentally years in the past.

In this example, the clock on node B is wrong, but the events are still sorted correctly, thanks to the Lamport clock:

| node | event | lamport | wall clock |
| -- | --   |  --     |  --  |
| A | started(eventId=FOO) | 5 | 10:00 |
| B  | stopped(endedWorkIds=[FOO]) | 20 | 08:00 |

The disadvantage of Lamport clocks is that between nodes that cannot communicate (network partition) it will order causally unrelated events arbitrarily.
Consider this example, which is totally possible _even with correct device clocks_:

| node | event | lamport | wall clock |
| -- | --   |  --     |  --  |
|   | (network partition starts) | 1 | 08:00 |
| A | started(eventId=BAR) | 8 | 16:00 |
| A | stopped(endedWorkIds=[BAR]) | 9 | 16:15 |
| B | started(eventId=FOO) | 19 | 10:00 |
| B | stopped(endedWorkIds=[FOO]) | 20 | 11:00 |
|   | (network partition heals) | 22 | 16:30 |

When the partition started, at 08:00 in the morning, both nodes agreed on lamport=1.
However, the Lamport clock then progressed faster for node B.
So the events that it logged at 10:00 and 11:00 in the morning were sorted after node A’s events from later in the same day.

This is the reason that `workStopped` events should explicitly declare the `workStarted` events they refer to.
Consider this example:

| node | event | lamport | wall clock |
| -- | --   |  --     |  --  |
|   | (network partition starts) | 1 | 08:00 |
| A | started(eventId=BAR) | 3 | 10:00 |
| B | started(eventId=FOO) | 5 | 16:00 |
| A | stopped(endedWorkIds=[BAR]) | 19 | 11:15 |
|   | (network partition heals) | 22 | 16:30 |

Without the explicit `endedWorkIds=[BAR]`, the partition healing at 16:30 would mistakenly set the state to `idle`, even though there actually is ongoing work that started at 16:00.

In case a network partition causes multiple open work IDs, they can still be closed within one `workStopped`, if all knowledge is available:

| node | event | lamport | wall clock |
| -- | --   |  --     |  --  |
|   | (network partition starts) | 1 | 08:00 |
| A | started(eventId=BAR) | 3 | 11:15 |
| B | started(eventId=FOO) | 5 | 10:00 |
|   | (network partition heals) | 20 | 16:30 |
| A | stopped(endedWorkIds=[FOO,BAR]) | 22 | 17:00 |

If you are familiar with Conflict-free Replicated Data Types, you will know a very similar concept: The Add-wins set.