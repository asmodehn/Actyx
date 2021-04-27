import {
  ActyxEvent,
  CancelSubscription,
  Metadata,
  OffsetMap,
  PendingEmission,
  TaggedEvent,
  Where,
} from './types'

/** Which clock to compare events by. Defaults to `Lamport`. @beta */
export enum EventOrder {
  /**
   * Comparison according to Lamport clock, which is a logical clock,
   * meaning it preserves causal order even when wall clocks on devices are off.
   *
   * On the flip-side, for any two events where neither is a cause of the other,
   * lamport-order may be different from timestamp-order, if the devices creating the events
   * where disconnected from each other at the time.
   */
  Lamport = 'lamport',

  /**
   * Comparison according to wall clock time logged at event creation.
   * If the system clock on a device is wrong, the event's timestamp will also be wrong. */
  Timestamp = 'timestamp',
}

/** Query for a fixed set of known events. @public */
export type RangeQuery = {
  /** Statement to select specific events. Defaults to `allEvents`. */
  query?: Where<unknown>

  /**
   * Starting point (exclusive) for the query. Everything up-to-and-including `lowerBound` will be omitted from the result. Defaults empty record.
   *
   * Events from sources not included in the `lowerBound` will be delivered from start, IF they are included in `upperBound`.
   * Events from sources missing from both `lowerBound` and `upperBound` will not be delivered at all.
   */
  lowerBound?: OffsetMap

  /**
   * Ending point (inclusive) for the query. Everything covered by `upperBound` (inclusive) will be part of the result.
   *
   * If a source is not included in `upperBound`, its events will not be included in the result.
   **/
  upperBound: OffsetMap

  /** Desired order of delivery. Defaults to 'Asc' */
  order?: 'Asc' | 'Desc'
}

/** Query for a set of events which is automatically capped at the latest available upperBound. @public */
export type AutoCappedQuery = {
  /**
   * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
   * Defaults to empty map, which means no lower bound at all.
   * Sources not listed in the `lowerBound` will be delivered in full.
   */
  lowerBound?: OffsetMap

  /** Statement to select specific events. Defaults to `allEvents`. */
  query?: Where<unknown>

  /** Desired order of delivery. Defaults to 'Asc' */
  order?: 'Asc' | 'Desc'
}

/** Subscription to a set of events that may still grow. @public */
export type EventSubscription = {
  /**
   * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
   * Defaults to empty map, which means no lower bound at all.
   * Sources not listed in the `lowerBound` will be delivered in full.
   */
  lowerBound?: OffsetMap

  /** Statement to select specific events. Defaults to `allEvents`. */
  query?: Where<unknown>

  /** Maximum chunk size. Note that new events will **always** be delivered ASAP, without waiting for chunks to fill up. */
  maxChunkSize?: number
}

/** Query for observeEarliest. @beta  */
export type EarliestQuery<E> = {
  /** Statement to select specific events. */
  query: Where<E>

  /**
   * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
   * Defaults to empty map, which means no lower bound at all.
   * Sources not listed in the `lowerBound` will be delivered in full.
   */
  lowerBound?: OffsetMap

  /** The order to find min/max for. Defaults to `Lamport`.  */
  eventOrder?: EventOrder
}

/** Query for observeLatest. @beta  */
export type LatestQuery<E> = EarliestQuery<E>

/**
 * A chunk of events, with lower and upper bound.
 * A call to `queryKnownRange` with the included bounds is guaranteed to return exactly the contained set of events.
 * A call to `subscribe` with the included `lowerBound`, however, may find new events from sources not included in the bounds.
 */
export type EventChunk = {
  /** The event data. Sorting depends on the request which produced this chunk. */
  events: ActyxEvent[]

  /** The lower bound of the event chunk, independent of its sorting in memory. */
  lowerBound: OffsetMap

  /** The upper bound of the event chunk, independent of its sorting in memory. */
  upperBound: OffsetMap
}

/** Functions that operate directly on Events. @public  */
export interface EventFns {
  /** Get the current latest offsets known locally. */
  currentOffsets: () => Promise<OffsetMap>

  /**
   * Get all known events between the given offsets, in one array.
   *
   * @param query       - `RangeQuery` object specifying the desired set of events.
   *
   * @returns A Promise that resolves to the complete set of queries events.
   */
  queryKnownRange: (query: RangeQuery) => Promise<ActyxEvent[]>

  /**
   * Get all known events between the given offsets, in chunks.
   * This is helpful if the result set is too large to fit into memory all at once.
   * The returned `Promise` resolves after all chunks have been delivered.
   *
   * @param query       - `RangeQuery` object specifying the desired set of events.
   * @param chunkSize   - Maximum size of chunks. Chunks may be smaller than this.
   * @param onChunk     - Callback that will be invoked with every chunk, in sequence.
   *
   * @returns A Promise that resolves when all chunks have been delivered to the callback.
   */
  queryKnownRangeChunked: (
    query: RangeQuery,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ) => Promise<void>

  /**
   * Query all known events that occured after the given `lowerBound`.
   *
   * @param query  - `OpenEndedQuery` object specifying the desired set of events.
   *
   * @returns An `EventChunk` with the result and its bounds.
   *          The contained `upperBound` can be passed as `lowerBound` to a subsequent call of this function to achieve exactly-once delivery of all events.
   */
  queryAllKnown: (query: AutoCappedQuery) => Promise<EventChunk>

  /**
   * Query all known events that occured after the given `lowerBound`, in chunks.
   * This is useful if the complete result set is potentially too large to fit into memory at once.
   *
   * @param query       - `OpenEndedQuery` object specifying the desired set of events.
   * @param chunkSize   - Maximum size of chunks. Chunks may be smaller than this.
   * @param onChunk     - Callback that will be invoked for each chunk, in sequence. Second argument is an offset map covering all events passed as first arg.
   *
   * @returns A `Promise` that resolves to updated offset-map after all chunks have been delivered.
   */
  queryAllKnownChunked: (
    query: AutoCappedQuery,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ) => Promise<OffsetMap>

  /**
   * Subscribe to all events fitting the `query` after `lowerBound`.
   * They will be delivered in chunks of at most 5000.
   * New events are delivered as they become known.
   * The subscription goes on forever, until manually cancelled.
   *
   * @param query      - `EventSubscription` object specifying the desired set of events.
   * @param onChunk    - Callback that will be invoked for each chunk, in sequence. Second argument is the updated offset map.
   *
   * @returns A function that can be called in order to cancel the subscription.
   */
  subscribe: (
    query: EventSubscription,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ) => CancelSubscription

  /**
   * Observe always the **earliest** event matching the given query.
   * If there is an existing event fitting the query, `onNewEarliest` will be called with that event.
   * Afterwards, `onNewEarliest` will be called whenever a new event becomes known that is older than the previously passed one.
   * Note that the 'earliest' event may keep updating as new events become known.
   *
   * @param query                - Query to select the set of events.
   * @param onNewEarliest        - Callback that will be invoked whenever there is a 'new' earliest event.
   *
   * @returns A function that can be called in order to cancel the subscription.
   *
   * @beta
   */
  observeEarliest: <E>(
    query: EarliestQuery<E>,
    onNewEarliest: (event: E, metadata: Metadata) => void,
  ) => CancelSubscription

  /**
   * Observe always the **latest** event matching the given query.
   * If there is an existing event fitting the query, `onNewLatest` will be called with that event.
   * Afterwards, `onNewLatest` will be called whenever a new event becomes known that is younger than the previously passed one.
   *
   * @param query                - Query to select the set of events.
   * @param onNewLatest          - Callback that will be invoked for each new latest event.
   *
   * @returns A function that can be called in order to cancel the subscription.
   *
   * @beta
   */
  observeLatest: <E>(
    query: EarliestQuery<E>,
    onNewLatest: (event: E, metadata: Metadata) => void,
  ) => CancelSubscription

  /**
   * Among all events matching the query, find one that best matches some property.
   * This is useful for finding the event that has `min` or `max` of something.
   * E.g. `shouldReplace = (candidate, cur) => candidate.meta.timestampMicros > cur.meta.timestampMicros` keeps finding the event with the highest timestamp.
   *
   * @param query         - Query to select the set of `candidate` events.
   * @param shouldReplace - Should `candidate` replace `cur`?
   * @param onReplaced    - Callback that is evoked whenever replacement happens, i.e. we found a new best match.
   *
   * @returns A function that can be called in order to cancel the subscription.
   */
  observeBestMatch: <E>(
    query: Where<E>,
    shouldReplace: (candidate: ActyxEvent<E>, cur: ActyxEvent<E>) => boolean,
    onReplaced: (event: E, metadata: Metadata) => void,
  ) => CancelSubscription

  /**
   * Apply a `reduce` operation to all events matching `query`, in no specific order.
   * This is useful for operations that are **commutative**, e.g. `sum` or `product`.
   *
   * @param query         - Query to select the set of events to pass to the reducer.
   * @param reduce        - Compute a new state `R` by integrating the next event.
   * @param initial       - Initial, neutral state, e.g. `0` for a `sum` operation.
   * @param onUpdate      - Callback that is evoked with updated results.
   *                        If a batch of events was applied, `onUpdate` will only be called once, with the final new state.
   *
   * @returns A function that can be called in order to cancel the subscription.
   */
  observeUnorderedReduce: <R, E>(
    query: Where<E>,
    reduce: (acc: R, event: E, metadata: Metadata) => R,
    initial: R,
    onUpdate: (result: R) => void,
  ) => CancelSubscription

  /**
   * Emit a number of events with tags attached.
   *
   * @param events - Events to emit.
   *
   * @returns        A `PendingEmission` object that can be used to register callbacks with the emission’s completion.
   */
  emit: (events: ReadonlyArray<TaggedEvent>) => PendingEmission
}