/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { greaterThan } from 'fp-ts/lib/Ord'
import { Event, Events, OffsetMap } from '../eventstore/types'
import { SnapshotScheduler } from '../store/snapshotScheduler'
import { EventKey, LocalSnapshot, StateWithProvenance, Timestamp } from '../types'

export type SerializedStateSnap = LocalSnapshot<string>

export type PendingSnapshot = Readonly<{
  snap: SerializedStateSnap
  tag: string
  timestamp: Timestamp
}>

export type Reducer<S> = {
  appendEvents: (
    events: Events,
    emit: boolean,
  ) => {
    snapshots: ReadonlyArray<PendingSnapshot>
    emit: StateWithProvenance<S>[]
  }

  setState: (state: SerializedStateSnap) => void
}

export const stateWithProvenanceReducer = <S>(
  onEvent: (oldState: S, event: Event) => S,
  initialState: SerializedStateSnap,
  snapshotScheduler: SnapshotScheduler,
  deserializeState?: (jsonState: unknown) => S,
): Reducer<S> => {
  const deserialize = deserializeState
    ? (s: string): S => deserializeState(JSON.parse(s))
    : (s: string): S => JSON.parse(s) as S

  const deserializeSnapshot = (snap: SerializedStateSnap): LocalSnapshot<S> => {
    const snapState = deserialize(snap.state)
    return { ...snap, state: snapState }
  }

  // Head is always the latest state known to us
  let head: LocalSnapshot<S> = deserializeSnapshot(initialState)

  const snapshotHead = (): SerializedStateSnap => ({
    ...head,
    state: JSON.stringify(head.state),
  })

  const clonedHead = () => deserializeSnapshot(snapshotHead())

  let queue = snapshotQueue()

  const snapshotEligible = (latest: Timestamp) => (snapBase: PendingSnapshot) =>
    snapshotScheduler.isEligibleForStorage(snapBase, { timestamp: latest })

  // Advance the head by applying the given event array between (i ..= iToInclusive)
  const advanceHead = (events: Events, fromIdxExclusive: number, toIdxInclusive: number) => {
    if (fromIdxExclusive > toIdxInclusive) {
      throw new Error(
        'cannot move head backwards!, from:' + fromIdxExclusive + ' to:' + toIdxInclusive,
      )
    }

    let i = fromIdxExclusive + 1

    let { state, eventKey, cycle } = head
    // Clone before modification -> need to clone nowehere else
    const offsets = { ...head.psnMap }

    while (i <= toIdxInclusive) {
      const ev = events[i]
      state = onEvent(state, ev)
      OffsetMap.update(offsets, ev)
      eventKey = ev

      i += 1
      cycle += 1
    }

    head = {
      state,
      psnMap: offsets,
      cycle,
      eventKey,
      horizon: head.horizon, // TODO: Detect new horizons from events
    }
  }

  const appendEvents: Reducer<S>['appendEvents'] = (events, emit) => {
    // FIXME: Arguments are a bit questionable, but we can’t change the scheduler yet, otherwise the FES-based tests start failing.
    const statesToStore = snapshotScheduler.getSnapshotLevels(head.cycle + 1, events, 0)

    let fromIdxExclusive = -1
    for (const toStore of statesToStore) {
      advanceHead(events, fromIdxExclusive, toStore.i)
      fromIdxExclusive = toStore.i

      queue.addPending({
        snap: snapshotHead(),
        tag: toStore.tag,
        timestamp: events[toStore.i].timestamp,
      })
    }

    advanceHead(events, fromIdxExclusive, events.length - 1)

    const snapshots =
      events.length > 0
        ? queue.getSnapshotsToStore(snapshotEligible(events[events.length - 1].timestamp))
        : []

    return {
      snapshots,
      // This is for all downstream consumers, so we clone.
      emit: emit ? [clonedHead()] : [],
    }
  }

  return {
    appendEvents,

    setState: snap => {
      if (eventKeyGreater(snap.eventKey, head.eventKey)) {
        // Time travel to future: Reset queue
        queue = snapshotQueue()
      } else {
        // Time travel to the past: All newer cached states are invalid
        queue.invalidateLaterThan(snap.eventKey)
      }

      head = deserializeSnapshot(snap)
    },
  }
}

const eventKeyGreater = greaterThan(EventKey.ord)

const snapshotQueue = () => {
  const queue: PendingSnapshot[] = []

  const addPending = (snap: PendingSnapshot) => queue.push(snap)

  const invalidateLaterThan = (cutOff: EventKey) => {
    while (queue.length > 0 && eventKeyGreater(queue[queue.length - 1].snap.eventKey, cutOff)) {
      queue.pop()
    }
  }

  const getSnapshotsToStore = (
    storeNow: (snapshot: PendingSnapshot) => boolean,
  ): ReadonlyArray<PendingSnapshot> => {
    const res = []

    while (queue.length > 0 && storeNow(queue[0])) {
      res.push(queue.shift()!)
    }

    return res
  }

  return {
    addPending,
    invalidateLaterThan,
    getSnapshotsToStore,
  }
}