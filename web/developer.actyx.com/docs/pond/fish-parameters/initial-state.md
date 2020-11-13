---
title: initialState
hide_table_of_contents: true
---

The `initialState` is the starting state of a Fish, before it receives its first event.

The object you pass as `initialState` will never be modified by the Pond. It will be cloned whenever
needed. To observe the current state, always use `Pond.observe()`.

## Creating the Initial State when some Data is Mandatory

Coming up with an initial state can be tricky.

Often you model entities that have a lot of mandatory information to them. In a traditional database, you
would mark the corresponding column as `NOT NULL`. In an event-based system like ActyxOS, you will
translate this into a "Creation Event" with a type that contains a field for each piece of mandatory
information:

```ts
type ProcessCreated = {
  eventType: 'ProcessCreated'

  // Unique identifier for the process
  processId: string

  // non-nullable fields to be filled for every created process
  name: string
  description: string
  /* .. etc .. */
}

type ProcessEvent = ProcessCreated | SomeOtherProcessEventTypes

// creating a process:
const processCreatedEvent: ProcessCreated = createProcess() // take info from somewhere

const ProcessTag = Tag<ProcessEvent>('process')
const ProcessCreatedTag = Tag<ProcessCreated>('ProcessCreated')

pond.emit(ProcessTag.withId(processCreatedEvent.id).and(ProcessCreatedTag), processCreatedEvent)
```

For the Fish representing this Process, you will want to reference it by id, and have all the information as part of its state.

```ts
type ProcessFishState = {
  // mandatory fields
  id: string
  name: string
  description: string
}

const makeProcessFish = (id: string): Fish<ProcessFishState, ProcessEvent> => ({
  where: ProcessTag.withId(id),

  initialState: // Tough! Where to get name, description etc. from?
  
  // etc.
})
```

There is no reason why the Fish of an unknown process should be created: The starting point is a
unique identifier that becomes known only from the `ProcessCreated` event. Still, filling the
mandatory fields in the initial state is a problem.

### Using a Union Type for the State

One solution to this is to use a union type marking the different situations.

```ts
type UnknownProcess = {
  stateType: 'unknown'
  id: string
  // nothing else
}

export type KnownProcess = {
  stateType: 'known'
  id: string
  name: string
  description: string
  // .. all the relevant fields ..
}

type ProcessFishState = UnknownProcess | KnownProcess

const onEvent = (state: ProcessFishState, event: ProcessEvent) => {
  if (event.eventType === 'ProcessCreated') {
    // Somehow create the "known" state from the event
    return makeKnownProcessState(event)
  }

  // Other events cannot be handled as long as the ProcessCreated event wasn’t seen
  if (state.stateType === 'unknown') {
    return state
  }

  // .. Normal handling of other events ..
}

const makeProcessFish = (id: string): Fish<ProcessFishState, ProcessEvent> => ({
  where: ProcessTag.withId(id),
  initialState: { stateType: 'unknown', id }
  onEvent,
  fishId: FishId.of('process', id, 1),
})
```

The mandatory fields are now mandatory in the correct place, but the Fish’s public API is bad:
Observers must handle states differently based on `stateType`.

We can fix the public API for observers by exposing a dedicated observation function:

```ts
export type Callback: (state: KnownProcess) => void

export const observeProcess = (id: string, pond: Pond, callback: Callback) => {
  const filteredCallback = (state: ProcessFishState) => {
    // Skip states of type UnknownProcess for outside observers.
    if (state.stateType === 'known') {
     callback(state)
    }
  }
  
  return pond.observe(makeProcessFish(id), filteredCallback)
}
```