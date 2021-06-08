/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { Observable } from 'rxjs'
import { EventsSortOrder, Milliseconds, NodeId, OffsetMap, Where } from '../types'
import { mockEventStore } from './mockEventStore'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { testEventStore, TestEventStore } from './testEventStore'
import { ConnectivityStatus, Events, OffsetsResponse, UnstoredEvents } from './types'
import { WebsocketEventStore } from './websocketEventStore'

/**
 * Get the store's node id.
 */
export type RequestNodeId = () => Observable<NodeId>

/**
 * Get the store's swarm connectivity status.
 * That is: Do we know about other nodes receiving the events we create?
 */
export type RequestConnectivity = (
  // FIXME: Rename this argument (if we keep it in v2)
  specialSources: ReadonlyArray<NodeId>,
  hbHistDelayMicros: number,
  reportEvery: Milliseconds,
  currentPsnHistoryDelay: number,
) => Observable<ConnectivityStatus>

/**
 * Request the full present of the store, so the maximum CONTIGUOUS offset for each source that the store has seen and ingested.
 * The store will NEVER deliver events across PSN gaps. So the 'present' signifies that which the store is willing to deliver to us.
 * If Offset=2 of some source never reaches our store, that source’s present will never progress beyond Offset=1 for our store.
 * Nor will it expose us to those events that lie after the gap.
 * This also returns the events per source which are pending replication to this node.
 */
export type RequestOffsets = () => Promise<OffsetsResponse>

/**
 * This method is only concerned with already persisted events, so it will always return a finite (but possibly large)
 * stream.
 * It is an ERROR to query with unbounded or future PSN.
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
export type DoQuery = (
  lowerBound: OffsetMap, // this is the lower bound, regardless of requested sort order.
  upperBound: OffsetMap,
  query: Where<unknown> | string,
  sortOrder: EventsSortOrder,
) => Observable<Events>

/**
 * This method is concerned with both persisted and future events, so it will always return an infinite stream.
 *
 * The returned event chunks can contain events from multiple sources. Any individual source will not time-travel.
 * There is not sorting between different sources, not even within a single chunk.
 *
 * Getting events up to a maximum event key can be achieved for a finite set of sources by specifying sort by
 * event key and aborting as soon as the desired event key is reached.
 */
export type DoSubscribe = (
  lowerBound: OffsetMap,
  query: Where<unknown> | string,
) => Observable<Events>

/**
 * Store the events in the store and return them as generic events.
 */
export type DoPersistEvents = (events: UnstoredEvents) => Observable<Events>

export type EventStore = {
  readonly offsets: RequestOffsets
  readonly query: DoQuery
  readonly subscribe: DoSubscribe
  readonly persistEvents: DoPersistEvents
  // TODO: Offer this API again.
  // readonly connectivityStatus: RequestConnectivity
}

const noopEventStore: EventStore = {
  subscribe: () => Observable.empty(),
  query: () => Observable.empty(),
  offsets: () => Promise.resolve({ present: {}, toReplicate: {} }),
  persistEvents: () => Observable.empty(),
  // connectivityStatus: () => Observable.empty(),
}

export const EventStore: {
  noop: EventStore
  ws: (multiplexedWebsocket: MultiplexedWebsocket) => EventStore
  mock: () => EventStore
  test: (nodeId?: NodeId, eventChunkSize?: number) => TestEventStore
} = {
  noop: noopEventStore,
  ws: (multiplexedWebsocket: MultiplexedWebsocket) => new WebsocketEventStore(multiplexedWebsocket),
  mock: mockEventStore,
  test: testEventStore,
}
