---
title: isReset
hide_table_of_contents: true
---

`isReset` is a function that identifies events with a special property: They _reset_ the state to
some value that only depends on this specific event and the [initial state](./initial-state). That
means no previous event has any bearing whatsoever on the state after the _reset event_ has been
applied.

When an event appears where `isReset` returns `true`, then the Pond will first reset the state to
the initial state, and then apply the event.

When an event appears that is older than the most recent reset event, it is simply ignored, rather
than causing time travel as it would normally do.

## Performance Implications

Suppose a stream of events recording the current value of something, like the temperature of a
room. To get at the current value of the room, we write a very simple Fish:

```ts
// Just set the state to the new value.
const onEvent = (_previousTemperature: number, current: TemperatureEvent) => current.temperature

const currentTemperatureFish = {
  initialState: NaN,
  onEvent,
  isReset: (current: TemperatureEvent) => true
  // etc.
}
```

Without defining `isReset`, we would lose a lot of performance due to

- going through all historic events

- time travelling when events occur out of order

By defining `isReset`, we disregard historic events and avoid unneccessary time travel.