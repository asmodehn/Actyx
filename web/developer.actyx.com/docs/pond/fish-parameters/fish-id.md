---
title: fishId
hide_table_of_contents: true
---

The `fishId` is the unique identifier of a Fish. It is used to implement caching, in order to
improve performance of Pond-based applications. Whenever a Fish is observed via `Pond.observe()` or
`Pond.run()`, and it has already been observed previously, we can avoid aggregating all its events
again: Instead we just build on the previous state.

## Constructing a good FishId

Because the `fishId` is used for caching, it’s important that if two Fish have the same `fishId`,
they are really the same fish. We have split the id into three parts to make it easy to conform to
this requirement:

### `entityType: string`

This should be a string describing what sort of thing the Fish expresses. It can be thought of as a namespace
for the string supplied as `name`.

An `entityType` for a Fish that represents a user might be "User" or "Workstation".

A Fish may represent physical things or abstract concepts — whatever is needed — just like classes and objects.

### `name: string`

The concrete thing represented by the Fish. In the example of a "User" Fish it would be just the
username or user-id.

Sometimes there is just one Fish of an `entityType`. In that case the name might be "singleton" or
similar.
  
### `version: number`

A counter for logic changes in the Fish. Caching for Fish is persistent on disk. So when the
_program code_ of a Fish is changed, it must also be considered a different Fish. Since "entityType"
and "name" will still be the same, there is a plain version counter to identify code changes.

You should never lower the version number.