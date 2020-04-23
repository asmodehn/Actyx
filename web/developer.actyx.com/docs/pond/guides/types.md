---
title: All those types
---

We have chosen TypeScript for a reason: fishes offer more type-safety than using the JSON-based untyped Event Service by itself.

The full signature of `FishType.of()` has four type parameters:

- `S` is the type of the internal state that the fish accumulates by processing events
- `C` is the type of commands accepted by the `onComamnd` handler
- `E` is the type of events emitted by the `onCommand` handler and understood by the `onEvent` handler
- `P` is the type of the observable state for the outside world

In particular, the full type of the chat room fish we have developed so far is

```typescript
const chatRoomFish: FishTypeImpl<string[], ChatRoomCommand, ChatRoomEvent, string[]>
```

This allows interactions with fishes via `Pond.feed` and `Pond.observe` to be checked for correctness by the TypeScript compiler.
Unfortunately, this type-checking does not extend to the subscription set of a fish since the subscribed event streams do not necessarily have typescript declarations — they may come from ActyxOS apps that are implemented directly on top of the [Event Service](/os/docs/event-service).

> Note
>
> In a future version ActyxOS will support the registration of event schemata for event streams, allowing types to be checked across devices and apps.
> This will include compile-time declarations for TypeScript as well as runtime checks for all events passed into the Event Service API.

Static type information also gives you some measure of control over the evolution of your event types:
when changing the definition of the event type, you and your team will see this explicitly so that you can carefully consider whether the changes will be backwards compatible, i.e. whether the changed fish code will be able to still understand the existing old events.

An event-sourced system like ActyxOS needs similar care as a widely used database when updating the data schema.
In our case it is not a table structure whose columns change, it is a set of events whose properties may change, or new events may be added and old ones deprecated.
With the current Actyx Pond infrastructure, it is necessary to retain compatibility with old events when making changes, i.e. old events will stay in the event log as they were and will still need to be understood by new app versions.

> Note
>
> In a future version ActyxOS and Actyx Pond will support the registration of schema migration handlers that transform an event stream from one schema version to another.
> With this, the app code can be modified to work with the new types and the translation of old events is done by the infrastructure, splitting the management of backwards compatibility from the evolution of the program code.

The next section addresses another concern that arises from the event-sourced nature of ActyxOS:
with an ever-growing event log, waking up a fish would take longer the longer it has existed.
This is addressed using snapshots.