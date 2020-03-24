/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import * as assert from 'assert'
import { last } from 'fp-ts/lib/Array'
import { fromNullable, none, Option, some } from 'fp-ts/lib/Option'
import { greaterThanOrEq, lessThan } from 'fp-ts/lib/Ord'
import { uniqWith } from 'ramda'
import { Observable } from 'rxjs'
import { MonoTypeOperatorFunction } from 'rxjs/interfaces'
import { concatMap, map, takeWhile, toArray } from 'rxjs/operators'
import { Psn, SubscriptionSet } from './'
import { EventStore } from './eventstore'
import { Event, OffsetMap, OffsetMapBuilder, PersistedEventsSortOrders } from './eventstore/types'
import { LatestSnapshots } from './latestSnapshots'
import log from './loggers'
import { SnapshotStore } from './snapshotStore'
import { StatePointers } from './statePointers'
import { SnapshotScheduler } from './store/snapshotScheduler'
import { EnvelopeFromStore } from './store/util'
import {
  EventKey,
  FishName,
  LocalSnapshot,
  OnEvent,
  Semantics,
  SnapshotFormat,
  StatePointer,
  StateWithProvenance,
  TaggedIndex,
} from './types'
import { lookup, runStats, takeWhileInclusive } from './util'
import { getInsertionIndex } from './util/binarySearch'

/**
 * ~~ HERE be Dragons ~~ Be aware that this is necessarily a complex piece of code, optimized for
 * both performance and (somewhat) clarity, to the detriment of elegance.
 *
 *
 * The fish event store has the purpose of storing events and lazily computing the current state
 * of the fish by applying the events in the right order. It also deals with semantic and local
 * snapshots.
 *
 * The fish event store needs to track the present for all sources that are relevant for the fish
 * so it can reload its state from the event store at any time. This present is distinct (lagging)
 * from the global present that is tracked in the event store, because an event can only become part
 * of the events of a fish event store once it is processed (possibly interleaved with commands) by
 * the fish jar.
 */
export interface FishEventStore<S, E> {
  /**
   * Processes events for this fish event store. Filtering has to be done on the outside.
   *
   * In the most straightfoward case, the new events will simply be integrated with the existing
   * local event buffer, and cached states invalidated as far as necessary.  However, we will often
   * detect additional conditions and persist these findings into private variables, for
   * currentState to act accordingly:
   *
   * - If local snapshots are used for this fish, and one of the new events lies before the latest
   *   local snapshot (all earlier events known at the time of snapshotting having been dropped from
   *   the local buffer), then we set this.shatterAsap. This will cause currentState to reinitialize
   *   our whole state, the next time it is called. Hence when this variable is set, processEvents
   *   will, as a performance optimization, also stop integrating new events into the buffer, and
   *   instead just aggregate their psns into shatterAsap.rehydrateUpTo, which is used as the upper
   *   event query bound after shattering.
   *
   * - If among the new events there is a new latest semantic snapshot, this.recomputeLocalSnapshots
   *   will be set and currentState is expected to do exactly this: Drop all existing local
   *   snapshots, and schedule as many as possible for recreation.
   *
   * - The newest semantic snapshot (if any) from the events will also be persisted into
   *   latestSnapshots.semantic. All earlier events will be dropped from the event buffer. That is,
   *   we guarantee that 1) no semantic snapshot will ever reach the local events buffer, 2) if
   *   latestSnapshots.semantic is set, it is the event immediately preceding the locally buffered
   *   events. baseState() helps currentState() treat it accordingly in the event aggregation, and
   *   becomeLocal() will always drop latestSnapshots.semantic (as part of the event log truncation
   *   procedure).
   *
   * This will be called internally from init, and externally from the fish jar for incorporating
   * foreign and own events.
   */
  readonly processEvents: (events: ReadonlyArray<EnvelopeFromStore<E>>) => boolean

  /**
   * Get the current state by running the event aggregation over the local event buffer
   * the function is only asynchronous due to piping the newly computed states through the
   * local snapshot storage logic.
   */
  readonly currentState: () => Observable<StateWithProvenance<S>>
  /**
   * The present for all events relevant to this store.
   *
   * Without semantic snapshots, this is conceptually the combined psn map of all events in the store.
   * With semantic snapshots, this gets truncated because for a semantic snapshot the historic events
   * do not matter anymore. So a store containing just a single event which is a semantic snapshot will
   * have a present computed from just that event.
   *
   * The present is all that is needed to recreate or reinitialize a fish event store, given access to
   * a store.
   */
  // readonly present: () => OffsetMap

  /**
   * Reinitialize the store for a given psnMap.
   *
   * Note that calling present() afterwards will not necessarily yield the same psn map, but will
   * return a smaller psn map which just contains psns for sources that have events that are relevant
   * for the subscription set of the store.
   */
  readonly init: (present: OffsetMap) => Observable<FishEventStore<S, E>>

  /**
   * Gets the current events in the event order as an array. This will not be the full event log when
   * one of the snapshot mechanisms is enabled. Used only for debugging at this time.
   */
  readonly currentEvents: () => ReadonlyArray<EnvelopeFromStore<E>>

  /**
   * Perform validation of the internal state of the store and return validation errors as strings.
   * Only useful for debugging.
   *
   * Note that the fish event store will only enforce internal invariants in currentState, so validate
   * should be called immediately after currentState with no intermediate calls to processEvents.
   */
  readonly validate: () => ReadonlyArray<string>
}

type SemanticSnapshot<E> = (env: EnvelopeFromStore<E>) => boolean

export const getOrderErrors = <T>(
  elems: T[],
  ord: (a: T, b: T) => number,
): ReadonlyArray<string> => {
  const errors: string[] = []

  for (let i = 0, length = elems.length; i < length - 1; i += 1) {
    if (ord(elems[i], elems[i + 1]) >= 0) {
      errors.push(`unordered :${[elems[i], elems[i + 1]].map(x => JSON.stringify(x)).join(',')}`)
    }
  }
  return errors
}

/**
 * In place merge sort of two ordered arrays. After calling this method, out
 * will be properly ordered according to ord.
 *
 * @param l array sorted according to ord. Will not be modified!
 * @param r array sorted according to ord. Will not be modified!
 * @param out array containing a concatenation of l and r. Will be modified in place!
 * @param ord order for l, r and out
 *
 * @returns the highest index at which out did not have to be changed
 */
export function mergeSortedInto<K>(
  l: ReadonlyArray<K>, // original events
  r: ReadonlyArray<K>, // new events
  out: K[], // original events concatenated with new events, to be modified in place
  ord: (a: K, b: K) => number, // order
): number {
  // out must be concatenation of l and r
  // out.length == l.length + a.length
  let li = 0
  let ri = 0
  let ro = l.length // index of ri element in out
  let i = 0
  let w = -1
  while (i < out.length) {
    if (li < l.length) {
      if (ri < r.length) {
        const o = ord(l[li], r[ri])
        if (o < 0) {
          // we are taking from l, so it could be that everything is still ok
          if (i === li) {
            // already at the right place. No need to assign
            w = i
          } else {
            out[i] = l[li]
          }
          li++
        } else if (o > 0) {
          out[i] = r[ri]
          ro++
          ri++
        } else {
          log.pond.error('Got the same event twice:', l[li])
          // getting a duplicate
          if (i === li) {
            // everything still fine
            w = i
          } else {
            // prefer the older event
            out[i] = l[li]
          }
          // now remove the duplicate entry from the `out` array and progress
          out.splice(ro, 1)
          li++
          ri++
        }
      } else {
        if (i === li) {
          w = i
        } else {
          out[i] = l[li]
        }
        li++
      }
    }
    // there does not need to be an else case, since when we are copying from
    // r while l is exhausted things are guaranteed to be in the right place already!
    i += 1
  }
  return w
}

/**
 * basically processEvents, but easier to test because the type parameters are free
 * @returns true if it leaves events without corresponding states behind
 */
export const addAndInvalidateState = <E>(
  events: E[],
  invalidateHigherThan: (i: number) => void,
  newEvents: ReadonlyArray<E>,
  eventOrder: (a: E, b: E) => number,
): boolean => {
  // If all new events are younger than all old events,
  // we can simply append the new events and return.
  if (events.length === 0 || eventOrder(events[events.length - 1], newEvents[0]) < 0) {
    events.push(...newEvents)
    return true
  }

  // temporary copy of old events, also in event order as per preconditions
  // this lives only inside this method, so it should be GC friendly
  const events0 = events.slice()

  events.push(...newEvents)

  // concatenate and sort
  const highestUnmoved = mergeSortedInto(events0, newEvents, events, eventOrder)

  const change = highestUnmoved + 1 !== events.length
  const timeTravel = highestUnmoved + 1 !== events0.length

  if (timeTravel) {
    // invalidate states
    invalidateHigherThan(highestUnmoved)
    log.pond.info('time travel to index', highestUnmoved, 'of', events.length)
  }

  return change
}

const eventKeyGeq = greaterThanOrEq(EventKey.ord)
const eventKeyLt = lessThan(EventKey.ord)
const removeBelowHorizon = <E>(
  events: ReadonlyArray<EnvelopeFromStore<E>>,
  horizon: EventKey | undefined,
): ReadonlyArray<EnvelopeFromStore<E>> => {
  if (horizon === undefined || events.length === 0) {
    return events
  }

  // Handle the most common cases first:
  // All events above horizon.
  if (eventKeyGeq(EventKey.fromEnvelope(events[0]), horizon)) {
    return events
  }

  // All events below horizon.
  if (eventKeyLt(EventKey.fromEnvelope(events[events.length - 1]), horizon)) {
    return []
  }

  // Binary-search the horizon inside the events.
  const sliceStart = getInsertionIndex(events, horizon, (e, hrz) =>
    EventKey.ord.compare(EventKey.fromEnvelope(e), hrz),
  )
  assert(
    sliceStart > 0 && sliceStart < events.length,
    'Expected binary search to yield an index inside the array.',
  )

  return events.slice(sliceStart)
}

// Information about a requested shattering and rehydrating
// at the next opportunity
type ShatterAsap = Readonly<{
  // How far back to invalidate existing local snapshots
  earliestKnownShatteringEvent: EnvelopeFromStore<{}>
  // The "present" we need to rehydrate up to, after shattering,
  // in order to preserve exactly-once delivery semantics.
  // "Present" here means the boundary that the outside (fishJar)
  // expects us to have reached already.
  rehydrateUpTo: OffsetMapBuilder
  // The snapshot we suppose we are going to shatter.
  snapshotToShatter: LocalSnapshot<{}>
}>

const mkShatterAsap = (
  firstEvent: EnvelopeFromStore<{}>,
  events: EnvelopeFromStore<{}>[],
  psnMapBuilder: OffsetMapBuilder,
  latestLocalSnap: LocalSnapshot<{}>,
): ShatterAsap => {
  const knownOffset = events.reduce((psnMap, evt) => includeEvent(psnMap, evt), psnMapBuilder)

  return {
    rehydrateUpTo: knownOffset,
    earliestKnownShatteringEvent: firstEvent,
    snapshotToShatter: latestLocalSnap,
  }
}

const envelopeEarlier = lessThan(EnvelopeFromStore.ord)
const updateShatterAsap = <E>(
  firstEvent: EnvelopeFromStore<E>,
  newEvents: ReadonlyArray<EnvelopeFromStore<E>>,
) => (s: ShatterAsap): ShatterAsap => ({
  earliestKnownShatteringEvent: envelopeEarlier(firstEvent, s.earliestKnownShatteringEvent)
    ? firstEvent
    : s.earliestKnownShatteringEvent,

  rehydrateUpTo: newEvents.reduce((psnMap, evt) => includeEvent(psnMap, evt), s.rehydrateUpTo),

  snapshotToShatter: s.snapshotToShatter,
})

export class FishEventStoreImpl<S, E> implements FishEventStore<S, E> {
  readonly events: EnvelopeFromStore<E>[] = []

  private statePointers: StatePointers<S, E> = new StatePointers<S, E>(this.snapshotScheduler)

  private shatterAsap: Option<ShatterAsap> = none
  private recomputeLocalSnapshots: boolean = false

  private readonly latestSnapshots: LatestSnapshots<S, E> = new LatestSnapshots<S, E>()

  currentEvents(): ReadonlyArray<EnvelopeFromStore<E>> {
    return this.events
  }

  constructor(
    readonly fish: FishInfo<S, E>,
    readonly eventStore: EventStore,
    readonly snapshotStore: SnapshotStore,
    readonly snapshotScheduler: SnapshotScheduler,
  ) {}

  // Since our contractors (snapshotStore, eventStore) all
  // accept/expect undefined over EventKey.zero, we also
  // default to undefined here. (Improves safety and clarity.)
  private horizon(): EventKey | undefined {
    return this.latestSnapshots.fromSemanticFromLocalOrDefault(
      EventKey.fromEnvelope,
      l => l.horizon,
      undefined,
    )
  }

  private baseState(): StateWithProvenance<S> {
    return this.latestSnapshots.fromSemanticFromLocalOrDefault(
      ss => ({
        state: this.fish.onEvent(this.fish.initialState, ss),
        psnMap: { [ss.source.sourceId]: ss.psn },
      }),
      l => l,
      { state: this.fish.initialState, psnMap: OffsetMap.empty },
    )
  }

  // ** Beginning of methods that mutate the internal state: **
  // reset, init, becomeLocal, applyEvents, processEvents and its subordinates:
  // ordinaryInsert, semanticSnapshotOrientedInsert, mergeInsertEvents, startOrContinueShattering

  private reset(): void {
    this.truncateBuffers()
    this.latestSnapshots.clear()
    this.shatterAsap = none
    this.recomputeLocalSnapshots = false
  }

  private truncateBuffers(): void {
    this.events.length = 0
    this.statePointers = new StatePointers<S, E>(this.snapshotScheduler)
  }

  /**
   * Tell the store to initialise itself from the given psn map.
   * @param present psn map that represents the present as far as all events this store is interested in are concerned.
   */
  init(present: OffsetMap): Observable<FishEventStore<S, E>> {
    this.reset()

    return getLatestLocalSnapshot(this.snapshotStore, this.fish).concatMap(base =>
      this.hydrateFromLocalSnapshot(base, present),
    )
  }

  private hydrateFromLocalSnapshot(
    base: Option<LocalSnapshot<S>>,
    present: OffsetMap,
  ): Observable<FishEventStore<S, E>> {
    logChunkInfo(this.fish.semantics, this.fish.fishName, base, present)
    this.latestSnapshots.local = base

    if (this.fish.isSemanticSnapshot !== undefined) {
      // Get events in reverse, meaning we will buffer everything
      const events$ = getEventsAfterLatestSemanticSnapshot(
        base,
        this.eventStore,
        this.fish,
        present,
        this.fish.isSemanticSnapshot,
      )
      return Observable.from(events$).map(events => {
        this.processEvents(events)
        return this
      })
    } else {
      const chunks$ = getEventsForwardChunked(base, this.eventStore, this.fish, present)

      if (!this.fish.snapshotFormat) {
        // No local snapshots defined, i.e. streaming hydration does not help with anything.
        return chunks$.reduce((res, chunk) => this.processEvents(chunk) || res, false).mapTo(this)
      }

      // Get events in forward order, meaning we can apply received chunks incrementally
      return chunks$
        .mergeScan(
          (newStatePending, chunk) => {
            const wantsStateNow = this.processEvents(chunk)
            const n = wantsStateNow || newStatePending
            // Possibly snapshot and truncate the event buffer.
            if (n && this.events.length > this.snapshotScheduler.minEventsForSnapshot) {
              return this.currentState().mapTo(false)
            } else {
              return Observable.of(n)
            }
          },
          false,
          1,
        )
        .last()
        .mapTo(this)
    }
  }

  private becomeLocal = (localSnapshotPtr: StatePointer<S, E>): void => {
    const localSnapshotIndex = localSnapshotPtr.i
    const { semantics, fishName } = this.fish
    if (log.pond.debug.enabled) {
      log.pond.debug(
        '%s/%s now based on local snapshot at %s - dropping %s events',
        semantics,
        fishName,
        EventKey.format(EventKey.fromEnvelope(localSnapshotPtr.finalIncludedEvent)),
        localSnapshotIndex + 1,
      )
    }

    const { state, psnMap } = localSnapshotPtr.state
    const eventKey = EventKey.fromEnvelope(localSnapshotPtr.finalIncludedEvent)
    const newLatestLocalSnapshot = some({
      state,
      psnMap,
      eventKey,
      horizon: this.horizon(), // take over horizon from previous base
      cycle: this.latestSnapshots.local.map(l => l.cycle).getOrElse(0) + localSnapshotIndex + 1, // increase cycle
    })

    this.latestSnapshots.local = newLatestLocalSnapshot
    // drop events including the event from which the local snapshot was generated!
    const keep = localSnapshotIndex + 1
    this.events.splice(0, keep)
    this.statePointers.shiftBack(keep)
    // Semantic snapshot (if present) is always the earliest event, so every local snapshot must drop it.
    // MUST be cleared AFTER initializing the new local snapshot, because it provides the horizon.
    this.latestSnapshots.semantic = none
  }

  /**
   * @param newEvents new events, already sorted by event order ascending; duplicates will be ingored
   * @returns true if calling `currentState()` is required
   */
  processEvents(newEvents: ReadonlyArray<EnvelopeFromStore<E>>): boolean {
    const newEventsSorted = this.assertSorted(newEvents)

    if (this.fish.isSemanticSnapshot !== undefined) {
      return this.semanticSnapshotOrientedInsert(newEventsSorted, this.fish.isSemanticSnapshot)
    } else {
      return this.ordinaryInsert(newEventsSorted)
    }
  }

  // TODO: Disable when are are really sure all FES inputs are sorted.
  private assertSorted(
    newEvents: ReadonlyArray<EnvelopeFromStore<E>>,
  ): ReadonlyArray<EnvelopeFromStore<E>> {
    if (newEvents.length < 2) {
      return newEvents
    }

    let prev = newEvents[0]
    for (let i = 1; i < newEvents.length; i++) {
      const nxt = newEvents[i]
      const cmp = EnvelopeFromStore.ord.compare(prev, nxt)
      if (cmp > 0) {
        log.pond.error('Unsorted event batch, at index', i, prev, nxt, newEvents)
        return [...newEvents].sort(EnvelopeFromStore.ord.compare)
      } else if (cmp === 0) {
        log.pond.error('Duplicate event inside batch', nxt)
        // Not gonna bother with performance here, as this REALLY should not happen.
        return this.assertSorted(uniqWith(EnvelopeFromStore.ord.equals)(newEvents))
      }
      prev = nxt
    }

    return newEvents
  }

  /**
   * @param newEvents new events, already sorted by event order ascending; duplicates will be ingored
   * @returns true if calling `currentState()` is required
   */
  private ordinaryInsert(newEvents: ReadonlyArray<EnvelopeFromStore<E>>): boolean {
    if (newEvents.length === 0) {
      return false
    }

    if (this.startOrContinueShattering(newEvents)) {
      return true
    }

    return this.mergeInsertEvents(newEvents)
  }

  /**
   * @param newEvents events to be inserted into this store; duplicates will be ignored
   * @returns true if it leaves events without corresponding states behind
   */
  private mergeInsertEvents(newEvents: ReadonlyArray<EnvelopeFromStore<E>>): boolean {
    // log.pond.info('mergeInsertEvents', this.fish.fishName, 'current buffer size', this.events.length)
    return addAndInvalidateState(
      this.events,
      i => this.statePointers.invalidateDownTo(i),
      newEvents,
      EnvelopeFromStore.ord.compare,
    )
  }

  /**
   * @param newEvents new events, already sorted by event order ascending; duplicates will be ingored
   * @param isSemanticSnap semantic snapshot identification function
   * @returns true if calling `currentState()` is required
   */
  private semanticSnapshotOrientedInsert(
    newEvents: ReadonlyArray<EnvelopeFromStore<E>>,
    isSemanticSnap: SemanticSnapshot<E>,
  ): boolean {
    const horizonFiltered = removeBelowHorizon(newEvents, this.horizon())

    const semanticSnapIndex = findLastIndex(horizonFiltered, isSemanticSnap)

    // Nothing at all special about this batch of events.
    if (semanticSnapIndex === -1) {
      return this.ordinaryInsert(horizonFiltered)
    }

    const eventsToAppend = horizonFiltered.slice(semanticSnapIndex)

    const dropped = horizonFiltered.length - eventsToAppend.length
    if (dropped > 0) {
      log.pond.debug(
        '%s events dropped because inside the chunk they appeared before the newfound semantic snapshot',
        dropped,
      )
    }

    // By checking this condition only now, we avoid shattering due to irrelevant events.
    if (this.startOrContinueShattering(eventsToAppend)) {
      return true
    }

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const ss = eventsToAppend.shift()!
    assert(isSemanticSnap(ss), 'Shifted event should have been a semantic snapshot at this point')

    this.latestSnapshots.semantic = some(ss)
    this.recomputeLocalSnapshots = true

    // New semantic snapshot always means:
    // - Naturally, all future states (if any) have become invalid
    // - All past states have become unneeded
    // I.e. we can drop all buffered states.
    this.statePointers = new StatePointers<S, E>(this.snapshotScheduler)

    const newHorizon = EventKey.fromEnvelope(ss)
    const oldEventsAboveHorizon = removeBelowHorizon(this.events, newHorizon)
    this.events.splice(0, this.events.length, ...oldEventsAboveHorizon)

    if (eventsToAppend.length > 0) {
      this.mergeInsertEvents(eventsToAppend)
    }

    return true
  }

  private startOrContinueShattering(newEvents: ReadonlyArray<EnvelopeFromStore<E>>): boolean {
    if (this.fish.snapshotFormat === undefined || this.latestSnapshots.local.isNone()) {
      return false
    }

    const firstEvent = newEvents[0]
    const firstEventKey = EventKey.fromEnvelope(firstEvent)
    const latestLocalSnap = this.latestSnapshots.local.value
    if (
      this.shatterAsap.isNone() &&
      EventKey.ord.compare(firstEventKey, latestLocalSnap.eventKey) < 0
    ) {
      const sp = this.statePointers.latestStored()
      const from = sp.fold(0, x => x.i)
      const psnMap = sp.fold(latestLocalSnap.psnMap, s => s.state.psnMap)

      this.shatterAsap = some(
        mkShatterAsap(firstEvent, this.events.slice(from), { ...psnMap }, latestLocalSnap),
      )
      // We will not need these buffers anymore until rehydration.
      this.truncateBuffers()
    }

    // When we know we are going to shatter, we do not have to
    // update this.events anymore; the only thing that matters is shatter.rehydrateUpTo.
    if (this.shatterAsap.isSome()) {
      this.shatterAsap = this.shatterAsap.map(updateShatterAsap(firstEvent, newEvents))
      return true
    }

    return false
  }

  // TODO: does this needs to be async, how big k could be?
  // log how much function execution takes
  // show loading mask while it is working
  // for later eventually chunk `k` or by time
  applyEvents(statesToStore: ReadonlyArray<TaggedIndex>): StateWithProvenance<S> {
    const startingPoint = this.statePointers.latestStored()

    const cachedStateWithProvenance = startingPoint.foldL(() => this.baseState(), x => x.state)

    // psnMap is a mutable psnMap that is updated while going through the event.
    // so we *need* to make a clone of latest cached psnMap so we don't accidentally modify it in place
    // Typescript does not protect us because of https://github.com/Microsoft/TypeScript/issues/13347
    // it is kind of the rust way with mut, but without a compiler safety net...
    let psnMap: OffsetMapBuilder = { ...cachedStateWithProvenance.psnMap }
    let state = cachedStateWithProvenance.state

    const cachedStates: StatePointer<S, E>[] = new Array(statesToStore.length)

    // Find the index of the first event to apply:
    // In case of a cached state, the state is aggregated including its `i`, so we must start at i+1.
    // In case of no cached state, the first event to apply is the one at index 0.
    let i = startingPoint.fold(0, x => x.i + 1)

    // Potentially another pointer at startingPoint got requested... (local snapshot logic)
    let ev = startingPoint.fold(this.events[0], x => x.finalIncludedEvent)

    // statesToStore are sorted in order of ascending `i`.
    for (let j = 0; j < statesToStore.length; j++) {
      const toStore = statesToStore[j]
      while (i <= toStore.i) {
        ev = this.events[i]
        state = this.fish.onEvent(state, ev)
        psnMap = includeEvent(psnMap, ev)

        i += 1
      }

      assert(
        // i has been incremented by 1 at the end of the loop, we actually do expect equality
        i - 1 === toStore.i,
        'Expected statesToStore to be in ascending order, with no entries earlier then the latestStored pointer.',
      )

      const psnMapCopy = { ...psnMap }
      const stateWithProvenance = { state, psnMap: psnMapCopy }

      cachedStates[j] = {
        ...toStore,
        state: stateWithProvenance,
        finalIncludedEvent: ev,
      }
    }

    this.statePointers.addPopulatedPointers(cachedStates)

    // The state pointers interface does not guarantee a pointer for the latest event,
    // so we may need to apply some more.
    while (i < this.events.length) {
      state = this.fish.onEvent(state, this.events[i])
      psnMap = includeEvent(psnMap, ev)
      i += 1
    }

    // No need to copy the final returned psnMap, since we made a copy initially.
    return { state, psnMap }
  }

  currentState(): Observable<StateWithProvenance<S>> {
    const { snapshotFormat } = this.fish

    if (log.pond.debug.enabled) {
      this.logCurrentStateInvocation()
    }

    if (snapshotFormat === undefined) {
      return this.currentStateNoSnapshotting()
    } else {
      return this.currentStateWithLocalSnapshots(snapshotFormat)
    }
  }

  private currentStateNoSnapshotting(): Observable<StateWithProvenance<S>> {
    const levels = this.statePointers.getStatesToCache(1, this.events)

    const state = this.applyEvents(levels)

    return Observable.of(state)
  }

  private currentStateWithLocalSnapshots(
    snapshotFormat: SnapshotFormat<S, any>,
  ): Observable<StateWithProvenance<S>> {
    const { semantics, fishName } = this.fish

    if (this.shatterAsap.isSome()) {
      return this.shatterAndRehydrate(this.shatterAsap.value)
    }

    let snapshotCycle
    let clearSnapshotsPromise
    if (this.recomputeLocalSnapshots) {
      clearSnapshotsPromise = this.snapshotStore.invalidateSnapshots(
        semantics,
        fishName,
        EventKey.zero,
      )

      this.recomputeLocalSnapshots = false
      snapshotCycle = 0
    } else {
      clearSnapshotsPromise = Promise.resolve()
      snapshotCycle = this.latestSnapshots.local.map(l => l.cycle).getOrElse(0)
    }
    const levels = this.statePointers.getStatesToCache(snapshotCycle + 1, this.events)

    const state = this.applyEvents(levels)

    const localSnapshotsToPersist = this.statePointers.getSnapshotsToPersist()

    const lastLocalSnapshot = last(localSnapshotsToPersist)

    // Synchronously becomeLocal, since otherwise JS’s weird async scheduler may cause
    // our underlying states to be mutated in the meantime.
    if (lastLocalSnapshot.isSome()) {
      this.becomeLocal(lastLocalSnapshot.value)
    }

    const storeSnapshots: Promise<void[]> = clearSnapshotsPromise.then(() => {
      const persist = localSnapshotsToPersist.map(ptr => {
        const level = ptr.tag
        const index = ptr.i
        const entry = ptr.state

        return this.serializeSnapshot(snapshotFormat, entry)
          .then(blob => {
            const key = EventKey.fromEnvelope(ptr.finalIncludedEvent)
            return this.storeSnapshot(key, entry.psnMap, snapshotFormat.version, level, index, blob)
          })
          .catch(err => {
            log.pond.error(err)
          })
      })
      return Promise.all(persist)
    })

    return Observable.from(storeSnapshots).mapTo(state)
  }

  private shatterAndRehydrate(shatter: ShatterAsap): Observable<StateWithProvenance<S>> {
    const { semantics, fishName } = this.fish

    const latestLocalSnapshot = shatter.snapshotToShatter

    assert(
      this.latestSnapshots.local.exists(l =>
        EventKey.ord.equals(latestLocalSnapshot.eventKey, l.eventKey),
      ),
      'Latest local snapshot changed since ShatterAsap was initialized, this may be dangerous.',
    )

    const firstEvent = shatter.earliestKnownShatteringEvent
    const invalidateFrom = EventKey.fromEnvelope(firstEvent)

    log.pond.info(
      'shattering snapshot because %s is before %s - base psn map is %j - horizon is %s',
      EventKey.format(invalidateFrom),
      EventKey.format(latestLocalSnapshot.eventKey),
      latestLocalSnapshot.psnMap,
      latestLocalSnapshot.horizon ? EventKey.format(latestLocalSnapshot.horizon) : 'undefined',
    )

    const haveSource =
      (latestLocalSnapshot.psnMap[firstEvent.source.sourceId] as Psn | undefined) !== undefined

    log.pond.debug(
      'new source: %s, event: %s %j',
      !haveSource,
      EventKey.format(invalidateFrom),
      firstEvent.source,
    )

    return Observable.defer(() =>
      Observable.from(this.snapshotStore.invalidateSnapshots(semantics, fishName, invalidateFrom))
        .concatMap(() => {
          // get base and chunks based on the same "present" as we have now, derived from the
          // current (shattered) base and the events on top of that. This is the only info we are
          // going to keep.
          //
          // this is pretty close to a complete reinitialization of the FES.
          return this.init(shatter.rehydrateUpTo)
            .pipe(runStats.profile.profileObservable(`shatter-getevents/${this.fish.semantics}`))
            .concatMap(() => {
              return this.currentState().pipe(
                runStats.profile.profileObservable(`shatter-compute/${this.fish.semantics}`),
              )
            })
        })
        .last(),
    )
  }

  serializeSnapshot(
    snapshotFormat: SnapshotFormat<S, any>,
    entry: StateWithProvenance<S>,
  ): Promise<unknown> {
    const { semantics, fishName } = this.fish

    return new Promise((resolve, reject) => {
      log.pond.debug('serializing state for %s/%s', semantics, fishName)
      try {
        const blob = snapshotFormat.serialize(entry.state)
        runStats.counters.add(`state-serialized/${semantics}`)
        resolve(blob)
      } catch (err) {
        reject(`Failed to serialize state of ${semantics}/${fishName}: ${JSON.stringify(err)}`)
      }
    })
  }

  storeSnapshot(
    key: EventKey,
    psnMap: OffsetMap,
    version: number,
    level: string,
    index: number,
    blob: unknown,
  ): Promise<void> {
    const { semantics, fishName } = this.fish
    log.pond.debug('storing snapshot for %s/%s', semantics, fishName)
    return this.snapshotStore
      .storeSnapshot(
        semantics,
        fishName,
        key,
        psnMap,
        this.horizon(),
        this.latestSnapshots.local.fold(0, l => l.cycle) + index + 1, // after offset 0 we got one more event
        version,
        level,
        blob,
      )
      .then(stored => {
        if (stored) {
          runStats.counters.add(`snapshot-stored/${semantics}`)
          return undefined
        } else {
          throw new Error(`Failed to store snapshot of ${semantics}/${fishName}`)
        }
      })
  }

  private logCurrentStateInvocation(): void {
    const { semantics, fishName } = this.fish
    const { events } = this

    const [baseType, baseKey]: [
      string,
      EventKey
    ] = this.latestSnapshots.fromSemanticFromLocalOrDefault(
      s => ['semantic', EventKey.fromEnvelope(s)],
      l => ['local', l.eventKey],
      ['none', EventKey.zero],
    )

    const baseToPrint = [baseType, EventKey.format(baseKey)]

    if (events.length === 0) {
      log.pond.debug(
        'call to currentState of %s/%s with base %j and 0 events',
        semantics,
        fishName,
        baseToPrint,
      )
    } else {
      log.pond.debug(
        'call to currentState of %s/%s with base %j and %d events %s..%s',
        semantics,
        fishName,
        baseToPrint,
        this.events.length,
        EventKey.format(EventKey.fromEnvelope(this.events[0])),
        EventKey.format(EventKey.fromEnvelope(this.events[this.events.length - 1])),
      )
    }
  }

  validate(): ReadonlyArray<string> {
    const errors: string[] = []
    errors.push(...getOrderErrors(this.events, EnvelopeFromStore.ord.compare))
    return errors
  }
}

const findLastIndex = <T>(es: ReadonlyArray<T>, p: (e: T) => boolean): number => {
  for (let i = es.length - 1; i >= 0; i--) {
    if (p(es[i])) {
      return i
    }
  }
  return -1
}

/**
 * Updates a given psn map with a new event.
 * Note that the events need to be applied in event order
 *
 * @param psnMap the psn map to update. WILL BE MODIFIED IN PLACE
 * @param ev the event to include
 */
const includeEvent = <E>(psnMap: OffsetMapBuilder, ev: EnvelopeFromStore<E>): OffsetMapBuilder => {
  const {
    psn,
    source: { sourceId },
  } = ev
  const current = lookup(psnMap, sourceId)
  if (current === undefined || current < psn) {
    psnMap[sourceId] = psn
  }
  return psnMap
}

/**
 * Groups a base and a number of event chunks on top of it.
 */
export type BaseAndChunks<S> = Readonly<{
  base: Option<LocalSnapshot<S>>
  chunks: Observable<ReadonlyArray<EnvelopeFromStore<any>>>
}>

export const getLatestLocalSnapshot = <S, E>(
  snapshotStore: SnapshotStore,
  fish: FishInfo<S, E>,
): Observable<Option<LocalSnapshot<S>>> => {
  const { semantics, fishName, snapshotFormat } = fish

  if (!snapshotFormat) {
    return Observable.of(none)
  }

  return Observable.defer(() =>
    snapshotStore.retrieveSnapshot(semantics, fishName, snapshotFormat.version),
  ).map(x => {
    runStats.counters.add(`snapshot-wanted/${semantics}`)
    return fromNullable(x).fold(none, localSnapshot => {
      runStats.counters.add(`snapshot-found/${semantics}`)
      const { eventKey, state: blob, psnMap, horizon, cycle } = localSnapshot
      try {
        const state: S = snapshotFormat.deserialize(blob)
        return some({ state, psnMap, eventKey, horizon, cycle })
      } catch {
        log.pond.error('failed to deserialize state of %s/%s', semantics, fishName)
        return none
      }
    })
  })
}

type EventFilterTransform = MonoTypeOperatorFunction<EnvelopeFromStore<any>>
export const getEventsAfterLatestSemanticSnapshot = async <S, E>(
  base: Option<LocalSnapshot<S>>,
  eventStore: EventStore,
  fish: FishInfo<S, E>,
  present: OffsetMap,
  isSemanticSnapshot: SemanticSnapshot<E>,
): Promise<EnvelopeFromStore<E>[]> => {
  const { subscriptionSet } = fish

  // filter transform for filtering by horizon. We are not interested in events at or below the horizon, so
  // takeWhile
  const horizonFilter = base
    .chain(x => fromNullable(x.horizon))
    .map<EventFilterTransform>(hzon =>
      takeWhile(ev => EventKey.ord.compare(EventKey.fromEnvelope(ev), hzon) > 0),
    )
  // filter transform for when we look for a semantic snapshot. We want the semantic snapshot to be the last
  // event to be returned, so takeWhileInclusive
  const ssFilter: EventFilterTransform = takeWhileInclusive(x => !isSemanticSnapshot(x))

  const fromExclusive = base.map(x => x.psnMap).getOrElse({})
  const horizon = base.chain(x => fromNullable(x.horizon)).getOrElse(EventKey.zero)

  const allEventsInOneChunk$ = eventStore
    .persistedEvents(
      { default: 'min', psns: fromExclusive },
      { default: 'min', psns: present },
      subscriptionSet,
      PersistedEventsSortOrders.ReverseEventKey,
      horizon,
    )
    .pipe(
      concatMap(envelopes => envelopes),
      // TODO: Provide decoder?
      map(x => Event.toEnvelopeFromStore<E>(x)),
      horizonFilter.getOrElse(x => x),
      ssFilter,
      // maybe use bufferCount here to avoid having a single large array? But currently it does not matter because the
      // fish event store will need them all anyway
      toArray(),
      // chunk contains events in reverse eventkey order, but fish event store needs them in ascending event key order
      map(chunk => chunk.reverse()),
    )

  return allEventsInOneChunk$.toPromise()
}

export const getEventsForwardChunked = <S, E>(
  base: Option<LocalSnapshot<S>>,
  eventStore: EventStore,
  fish: FishInfo<S, E>,
  present: OffsetMap,
): Observable<EnvelopeFromStore<E>[]> => {
  const { subscriptionSet } = fish

  const fromExclusive = base.map(x => x.psnMap).getOrElse({})

  const chunks = eventStore
    .persistedEvents(
      { default: 'min', psns: fromExclusive },
      { default: 'min', psns: present },
      subscriptionSet,
      PersistedEventsSortOrders.EventKey,
      undefined, // No semantic snapshots means no horizon, ever.
    )
    .map(x => {
      // TODO: Provide decoder?
      return x.map(ev => Event.toEnvelopeFromStore<E>(ev))
    })

  return chunks
}

const logChunkInfo = <T>(
  semantics: Semantics,
  fishName: FishName,
  base: Option<LocalSnapshot<T>>,
  present: OffsetMap,
): void => {
  if (log.pond.debug.enabled) {
    let newSources = 0
    let missingEvents = 0
    let missingEventsNewSources = 0

    const psnMap = base.map(x => x.psnMap).getOrElse({})
    const horizon = base.chain(x => fromNullable(x.horizon)).map(EventKey.format)

    Object.entries(present).forEach(([source, to]) => {
      const from = OffsetMap.lookup(psnMap, source)
      const count = to - from
      if (from === -1) {
        newSources += 1
        missingEventsNewSources += count
      } else {
        missingEvents += count
      }
    })
    log.pond.debug(
      'Getting chunks for %s/%s. Horizon %s. %s new sources, %s new possible events for existing sources, %s possible new events for new sources',
      semantics,
      fishName,
      horizon,
      newSources,
      missingEvents,
      missingEventsNewSources,
    )
  }
}

/**
 * Information about a live fish that is needed from inside the fish event store to do its job
 *
 * Some of this info is just copied over from the FishType, some of it identifies the specific
 * fish instance.
 */
export type FishInfo<S, E> = Readonly<{
  semantics: Semantics
  fishName: FishName
  subscriptionSet: SubscriptionSet
  initialState: S
  onEvent: OnEvent<S, E>
  isSemanticSnapshot: SemanticSnapshot<E> | undefined
  snapshotFormat: SnapshotFormat<S, any> | undefined
}>

/**
 * @param fish the fish info for the fish for which this is an event store
 * @param eventStore the event store instance
 * @param snapshotStore the snapshot store instance
 * @param snapshotScheduler the snapshot scheduler instance
 * @param present a psn map that identifies the present at the time of creation.
 *                Note that this can contain all sources, even though just a subset of them might
 *                be relevant for the fish.
 */
export const initialize = <S, E>(
  fish: FishInfo<S, E>,
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
  snapshotScheduler: SnapshotScheduler,
  present: OffsetMap,
): Observable<FishEventStore<S, E>> =>
  new FishEventStoreImpl(fish, eventStore, snapshotStore, snapshotScheduler).init(present)

export const FishEventStore = {
  initialize,
}