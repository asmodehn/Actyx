/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import * as t from 'io-ts'
import {
  DoPersistEvents,
  DoQuery,
  DoSubscribe,
  EventStore,
  RequestOffsets,
} from '../internal_common/eventStore'
import log from '../internal_common/log'
import {
  Event,
  EventIO,
  EventsSortOrders,
  OffsetsResponse,
  UnstoredEvents,
} from '../internal_common/types'
import { AppId, Where } from '../types'
import { EventKeyIO, OffsetMapIO } from '../types/wire'
import { validateOrThrow } from '../util'
import { MultiplexedWebsocket } from './multiplexedWebsocket'

export const enum RequestTypes {
  Offsets = 'offsets',
  Query = 'query',
  Subscribe = 'subscribe',
  Publish = 'publish',
}

const QueryRequest = t.readonly(
  t.type({
    lowerBound: OffsetMapIO,
    upperBound: OffsetMapIO,
    query: t.string,
    order: EventsSortOrders,
  }),
)

const SubscribeRequest = t.readonly(
  t.type({
    lowerBound: OffsetMapIO,
    query: t.string,
  }),
)

const PersistEventsRequest = t.readonly(t.type({ data: UnstoredEvents }))

const EventKeyWithTime = t.intersection([EventKeyIO, t.type({ timestamp: t.number })])
const PublishEventsResponse = t.type({ data: t.readonlyArray(EventKeyWithTime) })

const toAql = (w: Where<unknown> | string): string =>
  w instanceof String ? (w as string) : 'FROM ' + w.toString()

// TODO: Can we just change the type of MultiplexedWebsocket "payload" from unknown to this?
type TypedMsg = {
  type: string
}

export class WebsocketEventStore implements EventStore {
  constructor(private readonly multiplexer: MultiplexedWebsocket, private readonly appId: AppId) {}

  offsets: RequestOffsets = () =>
    this.multiplexer
      .request(RequestTypes.Offsets)
      .map(validateOrThrow(OffsetsResponse))
      .first()
      .toPromise()

  query: DoQuery = (lowerBound, upperBound, whereObj, sortOrder) =>
    this.multiplexer
      .request(
        RequestTypes.Query,
        QueryRequest.encode({
          lowerBound,
          upperBound,
          query: toAql(whereObj),
          order: sortOrder,
        }),
      )
      .filter(x => (x as TypedMsg).type === 'event')
      .map(validateOrThrow(EventIO))

  subscribe: DoSubscribe = (lowerBound, whereObj) =>
    this.multiplexer
      .request(
        RequestTypes.Subscribe,
        SubscribeRequest.encode({
          lowerBound,
          query: toAql(whereObj),
        }),
      )
      .filter(x => (x as TypedMsg).type === 'event')
      .map(validateOrThrow(EventIO))

  persistEvents: DoPersistEvents = events => {
    const publishEvents = events

    return this.multiplexer
      .request(RequestTypes.Publish, PersistEventsRequest.encode({ data: publishEvents }))
      .map(validateOrThrow(PublishEventsResponse))
      .map(({ data: persistedEvents }) => {
        if (publishEvents.length !== persistedEvents.length) {
          log.ws.error(
            'PutEvents: Sent %d events, but only got %d PSNs back.',
            publishEvents.length,
            events.length,
          )
          return []
        }
        return publishEvents.map<Event>((ev, idx) => ({
          ...persistedEvents[idx],
          appId: this.appId,
          tags: ev.tags,
          payload: ev.payload,
        }))
      })
      .defaultIfEmpty([])
  }
}
