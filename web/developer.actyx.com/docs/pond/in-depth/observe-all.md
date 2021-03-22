---
title: observeAll() Behavior
hide_table_of_contents: true
---

`Pond.observeAll()` goes from a set of _seed_events_ selected by `seedEventsSelector: Where<F>`
to a set of Fish created by applying `makeFish: (seedEvent: F) => Fish<S, any>` to the events.

The Fish are deduplicated based on their `fishId`.

All the created Fish are then observed, and the collected latest states passed as an array to the supplied `callback: (states: S[]) => void`.

For a general introduction to `observeAll`, [read our blog post.](/blog/2020/11/17/introducing-observe-all#observe-all-existing-things)

This document aims to cover all the details and corner cases.

## Expiry

In order to prevent the set of Fish from growing forever, you can set expiry options.  
Currently the Pond supports `expireAfterSeed`, which will remove Fish from the set after the event that spawned them has reached a certain age.  
For example, setting `expireAfterSeed: Milliseconds.fromDays(14)` would only show Fish where the seed event is two weeks old or younger.

Expiry is computed lazily: Expiry of a Fish in and of itself will not lead to a new invocation of the callback.
But _eventually_ an expired Fish will be removed from the set.

## Deduplication and Expiry

If several seed events map to Fish with the same `fishId`, one of them is selected, the others dropped.
However, the latest seed event is used as basis for expiry.

Consider this example:

```ts
type ChatRoomMsg = {
  channel: string
  message: string
}

const chatMsgTag = Tag<ChatRoomMsg>('chatmsg')

// The Fish does only depend on the channel, not on the message --
// hence two of these with same fishId will be identical in every way.
const makeChatRoomFish = (e: ChatRoomMsg) => ({
  fishId: FishId.of('chat room', e.channel, 1),
  where: chatMsgTag.withId(e.channel),
  onEvent: (state, event) => state.push(event.message),
  initialState: []
})

pond.observeAll(
  chatMsgTag,
  makeChatRoomFish,
  // We will only observe chat rooms that had a message posted within the last 2 days
  { expireAfterSeed: Milliseconds.fromDays(2) },
  callback
)
```

Here we consciously create identical Fish again and again; since they have the same fishId, they will get deduplicated.
At the same time, each duplication will count for the `expireAfterSeed` and potentially make the Fish last longer.  
In the end, we get the list of states of all rooms that had messages posted in the last 2 days.

## Subscriptions of Created Fish

A Fish created by `makeFish: (seedEvent: F) => Fish<S, any>` may subscribe (via its `where` field) to any event stream it likes.

There is no obligation to include the seed event in the actual subscription.

The subscription can also select events emitted before the seed event, and they will be delivered normally.
The seed event is still the logical first event: From it, the whole Fish is created.

## Caching of the Fish Set

By default, the set of selected Fish will be reconstructed with every call to `observeAll()`.
If you wish for the aggregation to keep on running in the background, so that later invocations are faster, there is the `caching` option:

```ts  
pond.observeAll(
  chatMsgTag,
  makeChatRoomFish,
  { caching: Caching.InProcess('all-chat-rooms') },
  callback
)
```

The string passed to `Caching.InProcess` is the cache key that would later be used to pick up the ongoing aggregation immediately.
Make sure to only use the same cache key where the logic is really completely the same.

## Caching of Individual Fish

All Fish created by `makeFish` pass through the normal Pond-internal cache of Fish.
As such, if a Fish is created that has the same `fishId` as one previously observed
– even if it was observed via `observe` or `observeOne` –
then the previously created Fish will be used.