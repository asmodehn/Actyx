---
title: observeOne() Behavior
hide_table_of_contents: true
---

`observeOne` is a variant of [`observeAll`](./observe-all) that is useful when you are looking for one specific Fish.

From the events selected by `seedEventSelector: Where<F>`, one will be chosen to pass to `makeFish: (seedEvent: F) => Fish<S, any>` and spawn the Fish.
The Fish is then observed and its state passed to the `callback: (newState: S) => void` whenever it changes.

For a general introduction to `observeOne`, [read our blog post.](/blog/2020/11/17/introducing-observe-all#observe-one-specific-thing)

This document aims to give some more detail.

## Deduplication and Caching

From the events yielded by `seedEventSelector: Where<F>`, only one is chosen, even if more match.
Application code must not rely on a specific one being chosen.

The Fish created by `makeFish` is checked against the Pond’s internal cache of Fish, like all Fish are:
So if `makeFish` returns a `Fish` with a `fishId` that is already known to the Pond
– even if that Fish was woken via `observe` or `observeAll` rather than `observeOne` –
the previously cached Fish will be used.

## Subscriptions of the Fish

Same as for `observeAll`, the Fish returned by `makeFish` can subscribe to events that are actually older than its seed event.
The seed event is still logically the first, since the whole Fish is built from it.