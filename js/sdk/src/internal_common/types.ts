/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { right } from 'fp-ts/lib/Either'
import { Ord, ordNumber, ordString } from 'fp-ts/lib/Ord'
import { Ordering } from 'fp-ts/lib/Ordering'
import * as t from 'io-ts'
import { EventsSortOrder, isString } from '../types'
import { Codecs, OffsetMapIO } from '../types/wire'

// EnumType Class
export class EnumType<A> extends t.Type<A> {
  public readonly _tag: 'EnumType' = 'EnumType'
  public enumObject!: object
  public constructor(e: object, name?: string) {
    super(
      name || 'enum',
      (u): u is A => Object.values(this.enumObject).some(v => v === u),
      (u, c) => (this.is(u) ? t.success(u) : t.failure(u, c)),
      t.identity,
    )
    this.enumObject = e
  }
}

// simple helper function
export const createEnumType = <T>(e: object, name?: string) => new EnumType<T>(e, name)

export const OffsetsResponse = t.readonly(
  t.type({
    present: t.readonly(OffsetMapIO),
    toReplicate: t.record(Codecs.StreamId, t.number),
  }),
)

export type OffsetsResponse = t.TypeOf<typeof OffsetsResponse>

const stringRA = t.readonlyArray(t.string)

type Tags = ReadonlyArray<string>
type TagsOnWire = ReadonlyArray<string> | undefined
const Tags = new t.Type<Tags, TagsOnWire>(
  'TagsSetFromArray',
  (x): x is Tags => x instanceof Array && x.every(isString),
  // Rust side for now expresses empty tag arrays as omitting the field
  (x, c) => (x === undefined ? right([]) : stringRA.validate(x, c)),
  // Sending empty arrays is fine, though
  x => x,
)

export const EventIO = t.type({
  offset: Codecs.Offset,
  stream: Codecs.StreamId,
  timestamp: Codecs.Timestamp,
  lamport: Codecs.Lamport,
  appId: Codecs.AppId,
  tags: Tags,
  payload: t.unknown,
})
export type Event = t.TypeOf<typeof EventIO>
const compareEvents = (a: Event, b: Event): Ordering => {
  const lamportOrder = ordNumber.compare(a.lamport, b.lamport)
  if (lamportOrder !== 0) {
    return lamportOrder
  }
  const sourceOrder = ordString.compare(a.stream, b.stream)
  if (sourceOrder !== 0) {
    return sourceOrder
  }
  return ordNumber.compare(a.offset, b.offset)
}

const eventsEqual = (a: Event, b: Event): boolean =>
  // compare numerical fields first since it should be faster
  a.lamport === b.lamport && a.offset === b.offset && a.stream === b.stream
/**
 * Order for events
 *
 * Order is [lamport, sourceId]
 * Events are considered equal when lamport, sourceId, psn are equal without considering
 * the content of the payload. Having two events that have these fields equal yet a different
 * payload would be a grave bug in our system.
 */
const ordEvent: Ord<Event> = {
  equals: eventsEqual,
  compare: compareEvents,
}
export const Event = {
  ord: ordEvent,
}

/**
 * A number of events, not necessarily from the same source
 */
export const Events = t.array(EventIO)
export type Events = t.TypeOf<typeof Events>

/**
 * A number of generated events, that are going to be written to the store.
 */
export const UnstoredEvent = t.readonly(
  t.type({
    tags: Tags,
    payload: t.unknown,
  }),
)

export type UnstoredEvent = t.TypeOf<typeof UnstoredEvent>
export const UnstoredEvents = t.readonlyArray(UnstoredEvent)
export type UnstoredEvents = t.TypeOf<typeof UnstoredEvents>

export const EventsSortOrders: EnumType<EventsSortOrder> = createEnumType<EventsSortOrder>(
  EventsSortOrder,
  'PersistedEventsSortOrders',
)
export type EventsSortOrders = t.TypeOf<typeof EventsSortOrders>

/**
 * Connectivity status type.
 */
export enum ConnectivityStatusType {
  FullyConnected = 'FullyConnected',
  PartiallyConnected = 'PartiallyConnected',
  NotConnected = 'NotConnected',
}

const FullyConnected = t.readonly(
  t.type({
    status: t.literal(ConnectivityStatusType.FullyConnected),
    inCurrentStatusForMs: t.number,
  }),
)

const PartiallyConnected = t.readonly(
  t.type({
    status: t.literal(ConnectivityStatusType.PartiallyConnected),
    inCurrentStatusForMs: t.number,
    specialsDisconnected: t.readonlyArray(t.string),
    swarmConnectivityLevel: t.number, // Percent*100, e.g. 50% would be 50, not 0.5
    eventsToRead: t.number,
    eventsToSend: t.number,
  }),
)

const NotConnected = t.readonly(
  t.type({
    status: t.literal(ConnectivityStatusType.NotConnected),
    inCurrentStatusForMs: t.number,
    eventsToRead: t.number,
    eventsToSend: t.number,
  }),
)

/**
 * The IO-TS type parser for ConnectivityStatus.
 * @public
 */
export const ConnectivityStatus = t.union([FullyConnected, PartiallyConnected, NotConnected])

/**
 * Current connectivity of the underlying Actyx node.
 * @public
 */
export type ConnectivityStatus = t.TypeOf<typeof ConnectivityStatus>

/* Other things */

/** Hook to run on store connection being closed. */
export type StoreConnectionClosedHook = () => void

/** Configuration for the WebSocket store connection. */
export type WsStoreConfig = Readonly<{
  /** url of the destination */
  url: string
  /** protocol of the destination */
  protocol?: string
  /** Hook, when the connection to the store is closed */
  onStoreConnectionClosed?: StoreConnectionClosedHook
  /** retry interval to establish the connection */
  reconnectTimeout?: number

  // todo timeouts?, heartbeats? etc.
}>