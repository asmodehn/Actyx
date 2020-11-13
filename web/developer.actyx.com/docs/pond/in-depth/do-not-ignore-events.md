---
title: Do Not Ignore Events
hide_table_of_contents: true
---

Events should be atomic pieces of information that are irrefutably true. Something _did_
happen. The Event is the record of it having happened. It’s always an **anti-pattern** to silently
ignore events in the `onEvent` function.

Let’s look at a simple example that comes up very often. There is a piece of work to be done, call
it Activity. Assume a large space of workers who may freely pick up activities and work on them.

On some ActyxOS application they record this: "I have started/stopped work on Activity FOO."

An Activity can be marked as Completed: All work done, no further work required.

```ts
enum EventType {
  WorkStarted = 'WorkStarted',
  WorkStopped = 'WorkStopped',
  Completed = 'Completed'
}

type WorkStarted = {
  eventType: EventType.WorkStarted
  workerId: string
  activityId: string
}

type WorkStopped = {
  eventType: EventType.WorkStopped
  workerId: string
  activityId: string
}

type Completed = {
  eventType: EventType.Completed
  activityId: string
}

type ActivityEvent = WorkStarted | WorkStopped | Completed
```

Here is a good way to aggregate these events into a state for the activity:

```ts
type ActivityState = {
  activeWorkerIds: Record<string, boolean>
  completed: boolean
}

const onEvent = (state, event) => {
  switch (event.type) {
    case 'WorkStarted': {
      state.activeWorkerIds[event.workerId] = true
      return state
    }

    case 'WorkStopped': {
      delete state.activeWorkerIds[event.workerId]
      return state
    }

    case 'Completed': {
      state.completed = true
      return state
    }
  }
}
```

The important point is: _Even if the activity is Completed, WorkStarted events are still handled normally._

Of course the application wants to avoid that users work on completed activities.  But an event is
something that has already happened! An event is not about intention; it’s already _truth_. Ignoring
the event does not accomplish anything, it does not stop the user from working on the activity.

Someone did not get the info about the task being completed already, and started working on
it. That’s a fact. We depend on the worker to recognize the issue, stop working, and properly log
that fact by creating a WorkStopped event. Otherwise we will never know how much that person actually
worked. Just that the activity was marked as Completed does not mean they stopped working. This is often
very important for booking time data into external systems.

Hence it is crucial that the application still offers the "Stop Working" button for users
that are working on completed activities. Or one might consider a more flashy alternative: A
warning box popping up, saying "Please stop work on already completed activity! (tap HERE when you have
stopped)"