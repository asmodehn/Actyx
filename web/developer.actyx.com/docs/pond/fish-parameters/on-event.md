---
title: onEvent
hide_table_of_contents: true
---

`onEvent` is the function used to aggregate _events_ into _state_. Conceptionally it is very similar
to the function you pass to
[Array.reduce()](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/reduce). Think
of the sorted event log as the array, and the state as the accumulated object.

## Time Travel and Keeping `onEvent` Pure

The most important thing to keep in mind when implementing `onEvent` is that it may be called again
and again with very similar inputs. This is due to time travel. When an event _from the past_
arrives, it is inserted into its proper spot in the log of all relevant events. Then, in order to
compute an updated correct state, the `onEvent` aggregation is run over the event log again.

[Read a concrete example in our tutorial.](/docs/pond/guides/time-travel)

Due to this, it is very important that `onEvent` is a _pure function_. A pure function is a function
where the output depends solely on the inputs.

### Impure Procedures To Avoid

The following are examples of code that is NOT pure and hence must be avoided inside `onEvent`.

- Looking at the current time via `new Date()` or similar. If you need to get the time at which an
  event occured, look at the `metadata`.
  
- Accessing dynamic global state.

- Modifying anything that is not part of the output state, for example a variable captured by the
  `onEvent` function.

## The Inputs to `onEvent`

### `state: S`

The current state of the Fish. That is, a state to which all _previous_ events have already been
applied.  
Note that it will always just be the **locally known** previous events that have been applied. That
is the exact point of time travel: Some previous events may always [be yet
unknown](../in-depth/eventual-consistency).
  
### `event: E`

The current event to apply.

### `metadata: Metadata`

A collection of various metadata tied to the event.

- `isLocalEvent` - Whether the event was created on the same node that `onEvent` is currently being
  executed on.

- `tags` - The tags that were attached to the event when it was emitted.

- `timestampMicros` - **Microseconds** since the [Epoch](https://en.wikipedia.org/wiki/Unix_time) on the
  node that emitted the event, at time of emission. If the clock on that node was not set correctly, this timestamp
  will also be wrong.

- `timestampAsDate` - A function that returns the `timestampMicros` converted to a plain JS `Date`
  object.

- `lamport` - Timestamp according to the [Lamport
  Clock](https://en.wikipedia.org/wiki/Lamport_timestamp). This is only useful for debugging. Events
  are fed ordered by Lamport timestamp ascending.
  
- `eventId` - A unique identifier for the event. Every event has exactly one `eventId` which is unique
  to it, guaranteed to not collide with any other event.

## The Output of `onEvent`

The `onEvent` implementation must return a value of type `S`. The following are all legal:

- Returning the input state, unchanged. Although note that you [should not ignore events](../in-depth/do-not-ignore-events).

- Modifying the input state and returning it.

- Returning a completely new object.

The returned value will then be fed as input `state` to the `onEvent` invocation for the next
event.  
It will also be potentially published to observers that have called `pond.observe` for this
Fish. It’s important to note, however, that during time travel, observers are not notified of
intermediate states – they are only notified of the updated new latest state.  
A similar thing happens when starting observation on a new Fish that already has some events: All
existing events are applied, and then the observer receives the latest state. No intermediate
states are passed to the observation callback.

## The Order of Events - What does "previous" mean?

ActyxOS uses a specialized mechanism called [Lamport
Clock](https://en.wikipedia.org/wiki/Lamport_timestamp) to sort events. Effectively this works like
sorting by time, only better. Sorting by wall clock time would run into issues when clocks on devices
are off: It would require a robust NTP setup to keep time in sync.

Lamport timestamps work independently from device time. They also do a good job of preserving a
useful, logical order even when network partitions happen.

In the rare case that lamport timestamp of two events should be identical, other factors are used to
decide on a consistent ordering. Ultimately, the ordering of `eventId` (inside `metadata`) is
what decides the ordering of events.