---
title: deserializeState
hide_table_of_contents: true
---

A Fish’s state is frequently converted to JSON and back by the Pond, internally. Hence, transforming
the state to a JSON-string and back must result in an object identical to the original state.

So when you are using something as your state that can _not_ be trivially JSON-stringified, you need
to implement `deserializeState`. The process is the following:

- When transforming to JSON, the Pond just calls `JSON.stringify(state)`. Objects that don’t serialize
  directly to JSON must implement
  [toJSON](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/JSON/stringify#toJSON_behavior).

- When transforming back from JSON, the Pond calls `deserializeState(JSON.parse(jsonString))`. So
  the input to `deserializeState` is the object returned by `toJSON`, and `deserializeState` must
  convert that back into the proper state type.
  
## An Example with immutable-js

[immutable-js](https://github.com/immutable-js/immutable-js) is a great library for performant
immutable data types in JavaScript. In this example we aggregate received events into a `List`
object as defined by immutable-js:

```ts
import { List } from 'immutable'

// `push` returns a new `List` with the given item appended
const onEvent = (state: List<unknown>, event: unknown): List<unknown> => state.push(event)
```

Since the data structures provided by immutable-js already implement `toJSON`, the serialization
works fine: The `List` becomes a plain JS array. (If you use your own data structures or classes
that cannot be serialized, you have to implement `toJSON` yourself.)

Now when the state is parsed, it is still an array and not a `List`. Hence we implement
`deserializeState`:

```ts
// List has a constructor function that takes an array
const deserializeState = (stateFromJson: unknown): List<unknown> => List(stateFromJson as unknown[])

const fish = {
  fishId: FishId.of('list-of-all-events', 'immutable-js', 1),
  onEvent,
  initialState: List(),
  where: allEvents,
  deserializeState,
}
```