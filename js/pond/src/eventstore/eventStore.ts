/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Observable } from 'rxjs'
import { SubscriptionSet } from '../subscription'
import { EventKey, SourceId, Milliseconds } from '../types'
import { mockEventStore } from './mockEventStore'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { testEventStore } from './testEventStore'
import {
  AllEventsSortOrder,
  ConnectivityStatus,
  Events,
  OffsetMapWithDefault,
  PersistedEventsSortOrder,
  UnstoredEvents,
} from './types'
import { WebsocketEventStore } from './websocketEventStore'

/**
 * Get the store's source id.
 */
export type RequestSourceId = () => Observable<SourceId>

/**
 * Get the store's swarm connectivity status.
 * That is: Do we know about other nodes receiving the events we create?
 */
export type RequestConnectivity = (
  specialSources: ReadonlyArray<SourceId>,
  hbHistDelayMicros: number,
  reportEvery: Milliseconds,
  currentPsnHistoryDelay: number,
) => Observable<ConnectivityStatus>

/**
 * Request the full present of the store, so the maximum CONTIGUOUS psn for each source that the store has seen and ingested.
 * The store will NEVER deliver events across PSN gaps. So the 'present' signifies that which the store is willing to deliver to us.
 * If PSN=2 of some source never reaches our store, that source’s present will never progress beyond PSN=1 for our store.
 * Nor will it expose us to those events that lie after the gap.
 */
export type RequestPresent = () => Observable<OffsetMapWithDefault>

/**
 * Request the highest seen offsets from the store, not all of which may be available (gapless) yet.
 * Note that this is purely for diagnostical purposes combined with the 'present'.
 * 'Highest seen' cannot be used to drive logic which queries events, because the store does not deliver
 * across PSN gaps, while 'highest seen' reports highest seen irregardless of gaps.
 */
export type RequestHighestSeen = () => Observable<OffsetMapWithDefault>

/**
 * This method is only concerned with already persisted events, so it will always return a finite (but possibly large)
 * stream.
 * It is an ERROR to request reverse order with unbounded or future PSN.
 * It is probably a user error to ask for an unbounded set of sources or for events with higher PSN than in the 'present'.
 *
 * The returned event chunks can contain events from multiple sources. If the sort order is unsorted, we guarantee
 * that chunks will be sorted by ascending psn for each source.
 *
 * Looking for a semantic snapshot can be accomplished by getting events in reverse event key order and aborting the
 * iteration as soon as a semantic snapshot is found.
 *
 * Depending on latency between pond and store the store will traverse into the past a bit further than needed, but
 * given that store and pond are usually on the same machine this won't be that bad, and in any case this is perferable
 * to needing a way of sending a javascript predicate to the store.
 */
export type RequestPersistedEvents = (
  fromPsnsExcluding: OffsetMapWithDefault, // 'from' is the lower bound, regardless of requested sort order.
  toPsnsIncluding: OffsetMapWithDefault,
  subscriptionSet: SubscriptionSet,
  sortOrder: PersistedEventsSortOrder,
  minEventKey?: EventKey,
) => Observable<Events>

/**
 * This method is concerned with both persisted and future events, so it can return an infinite stream if the toPsn
 * is in the future.
 *
 * The returned event chunks can contain events from multiple sources. If the sort order is unsorted, we guarantee
 * that chunks will be sorted by ascending psn for each source: any individual source will not time-travel.
 *
 * A sort order of anything else but unsorted mostly makes sense if either fromPsnsExcluding or toPsnsIncluding
 * constrains the set of sources to a finite set. When asking for sorted events for an infinite set of sources,
 * new sources will be included as they become known, but in a manner that the overall stream still remains sorted.
 * That is, for a new source, all its events older than the latest event emitted by this stream will NOT BE INCLUDED.
 *
 * When having a finite set of sources, sorting by event key can be used to get events in such a way that there
 * will be no time travel. Progress of the stream will be blocked as long as at least 1 source is disconnected.
 *
 * Getting events up to a maximum event key can be achieved for a finite set of sources by specifying sort by
 * event key and aborting as soon as the desired event key is reached.
 */
export type RequestAllEvents = (
  fromPsnsExcluding: OffsetMapWithDefault, // 'from' is the lower bound, regardless of requested sort order.
  toPsnsIncluding: OffsetMapWithDefault,
  subscriptionSet: SubscriptionSet,
  sortOrder: AllEventsSortOrder,
  minEventKey?: EventKey,
) => Observable<Events>

/**
 * Store the events in the store and return them as generic events.
 */
export type RequestPersistEvents = (events: UnstoredEvents) => Observable<Events>

/**
 * publish a log message via the event store.
 */

export type EventStore = {
  readonly sourceId: SourceId
  readonly present: RequestPresent
  readonly highestSeen: RequestHighestSeen
  readonly persistedEvents: RequestPersistedEvents
  readonly allEvents: RequestAllEvents
  readonly persistEvents: RequestPersistEvents
  readonly connectivityStatus: RequestConnectivity
}

const noopEventStore: EventStore = {
  allEvents: () => Observable.empty(),
  persistedEvents: () => Observable.empty(),
  present: () => Observable.of({ psns: {}, default: 'max' as 'max' }),
  highestSeen: () => Observable.of({ psns: {}, default: 'max' as 'max' }),
  persistEvents: () => Observable.empty(),
  sourceId: SourceId.of('NoopSourceId'),
  connectivityStatus: () => Observable.empty(),
}

export const EventStore = {
  noop: noopEventStore,
  ws: (multiplexedWebsocket: MultiplexedWebsocket, sourceId: SourceId) =>
    new WebsocketEventStore(multiplexedWebsocket, sourceId),
  mock: mockEventStore,
  test: testEventStore,
}
