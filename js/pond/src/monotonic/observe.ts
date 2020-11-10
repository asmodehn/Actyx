/* eslint-disable @typescript-eslint/no-explicit-any */
import { clone } from 'ramda'
import { Observable, Scheduler } from 'rxjs'
import { Event, EventStore, OffsetMap } from '../eventstore'
import log from '../loggers'
import { PondStateTracker } from '../pond-state'
import { SnapshotStore } from '../snapshotStore'
import { SubscriptionSet } from '../subscription'
import {
  EventKey,
  FishId,
  IsReset,
  Metadata,
  SourceId,
  StateWithProvenance,
  toMetadata,
} from '../types'
import { eventsMonotonic, EventsMsg, MsgType, StateMsg } from './endpoint'
import { PendingSnapshot, SerializedStateSnap, stateWithProvenanceReducer } from './reducer'

// Take some Fish parameters and combine them into a "simpler" onEvent
// with typical reducer signature: (S, E) => S
const mkOnEventRaw = <S, E>(
  sourceId: SourceId,
  initialState: S,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  isReset?: IsReset<E>,
) => {
  const metadata = toMetadata(sourceId)

  if (!isReset) {
    return (state: S, ev: Event) => {
      const m = metadata(ev)
      const payload = ev.payload as E

      return onEvent(state, payload, m)
    }
  }

  return (state: S, ev: Event) => {
    const m = metadata(ev)
    const payload = ev.payload as E

    if (isReset(payload, m)) {
      return onEvent(clone(initialState), payload, m)
    } else {
      return onEvent(state, payload, m)
    }
  }
}

/*
 * Observe a Fish using the subscribe_monotonic endpoint (currently TS impl., but can drop in real impl.)
 *
 * Signature is the same as FishJar.hydrateV2 so we can easily swap it in.
 */
export const observeMonotonic = (
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
  _pondStateTracker: PondStateTracker,
) => <S, E>(
  subscriptionSet: SubscriptionSet,
  initialState: S,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  fishId: FishId,
  isReset?: IsReset<E>,
  deserializeState?: (jsonState: unknown) => S,
): Observable<StateWithProvenance<S>> => {
  const endpoint = eventsMonotonic(eventStore, snapshotStore)

  const { sourceId } = eventStore

  const onEventRaw = mkOnEventRaw(sourceId, clone(initialState), onEvent, isReset)

  const initialStateAsString = JSON.stringify(initialState)
  // Here we can find earlier states that we have cached in-process.
  // Returning the initial state is always fine, though. It just leads to more processing.
  const findStartingState = (_before: EventKey): SerializedStateSnap => ({
    state: initialStateAsString,
    psnMap: {},
    cycle: 0,
    eventKey: EventKey.zero,
    horizon: undefined,
  })

  // Create a message that sets the Reducer back to a locally cached state.
  const makeResetMsg = (trigger: EventKey): StateMsg => {
    const latestValid = findStartingState(trigger)
    return {
      type: MsgType.state,
      snapshot: latestValid,
    }
  }

  const storeSnapshot = async (toStore: PendingSnapshot) => {
    const { snap, tag } = toStore
    snapshotStore.storeSnapshot(
      fishId.entityType,
      fishId.name,
      snap.eventKey,
      snap.psnMap,
      snap.horizon,
      snap.cycle,
      fishId.version,
      tag,
      snap.state,
    )
  }
  // Chain of snapshot storage promises
  let storeSnapshotsPromise: Promise<void> = Promise.resolve()

  const trackingId = FishId.canonical(fishId)

  // The stream of update messages.
  // This is a transformation from the endpoint’s protocol, which includes time travel,
  // to a protocal that does NOT terminate and not send time travel messages:
  // Rather, time travel messages are mapped to a restart of the stream.
  // In the end we get an easier to consume protocol.
  const updates = (from?: OffsetMap): Observable<StateMsg | EventsMsg> => {
    const stream = () =>
      endpoint(fishId, subscriptionSet, from)
        // This is a high-pressure pipeline with potential recursion, hence we run on a Scheduler
        // to guard against excess CPU usage and stack overflow.
        .subscribeOn(Scheduler.queue)
        .concatMap(msg => {
          if (msg.type === MsgType.timetravel) {
            const resetMsg = makeResetMsg(msg.trigger)
            const startFrom = resetMsg.snapshot.psnMap

            log.pond.info(trackingId, 'time traveling due to', EventKey.format(msg.trigger))

            // On time travel, reset the state and start a fresh stream
            return Observable.concat(
              Observable.of(resetMsg),
              // Recursive call, can’t be helped
              updates(OffsetMap.isEmpty(startFrom) ? undefined : startFrom),
            )
          }

          return [msg]
        })
        .catch(err => {
          log.pond.error(err)

          // Reset the reducer and let the code further below take care of restarting the stream
          return Observable.of(makeResetMsg(EventKey.zero))
        })

    // Wait for pending snapshot storage requests to finish
    return Observable.from(storeSnapshotsPromise)
      .first()
      .concatMap(stream)
  }

  // If the stream of updates terminates without a timetravel message – due to an error or the ws engine –,
  // then we can just restart it. (Tests pending.)
  const updates$ = Observable.concat(updates(), Observable.defer(updates))

  // This will probably turn into a mergeScan when local snapshots are added
  const reducer = stateWithProvenanceReducer(
    onEventRaw,
    {
      state: initialStateAsString,
      psnMap: OffsetMap.empty,
      eventKey: EventKey.zero,
      horizon: undefined,
      cycle: 0,
    },
    deserializeState,
  )
  return updates$.concatMap(msg => {
    switch (msg.type) {
      case MsgType.state: {
        log.pond.info(
          trackingId,
          'directly setting state.',
          'Num sources:',
          Object.keys(msg.snapshot.psnMap).length,
          '- Cycle:',
          msg.snapshot.cycle,
        )
        reducer.setState(msg.snapshot)
        return []
      }

      case MsgType.events: {
        log.pond.debug(
          trackingId,
          'applying event chunk of size',
          msg.events.length,
          '- caughtUp:',
          msg.caughtUp,
        )
        const s = reducer.appendEvents(msg.events, msg.caughtUp)
        storeSnapshotsPromise = storeSnapshotsPromise.then(async () => {
          await Promise.all(s.snapshots.map(storeSnapshot)).catch(log.pond.warn)
          return
        })
        return s.emit
      }
    }
  })
}
