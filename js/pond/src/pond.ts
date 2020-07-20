/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import { Observable, Scheduler } from 'rxjs'
import { CommandInterface } from './commandInterface'
import { EventStore } from './eventstore'
import { MultiplexedWebsocket } from './eventstore/multiplexedWebsocket'
import { TestEventStore } from './eventstore/testEventStore'
import { ConnectivityStatus, Events, UnstoredEvents } from './eventstore/types'
import { mkMultiplexer } from './eventstore/utils'
import { getSourceId } from './eventstore/websocketEventStore'
import { CommandPipeline, FishJar } from './fishJar'
import log from './loggers'
import { mkPondStateTracker, PondState, PondStateTracker } from './pond-state'
import { SnapshotStore } from './snapshotStore'
import { Config as WaitForSwarmConfig, SplashState } from './splashState'
import { Monitoring } from './store/monitoring'
import { SubscriptionSet, subscriptionsToEventPredicate } from './subscription'
import { toWireFormat, TypedTagIntersection } from './tagging'
import {
  CancelSubscription,
  EmissionRequest,
  Emit,
  Fish,
  FishId,
  FishName,
  IsReset,
  Metadata,
  Milliseconds,
  PendingEmission,
  Reduce,
  Semantics,
  Source,
  SourceId,
  StateEffect,
  StateWithProvenance,
  Timestamp,
} from './types'

const isTyped = (
  e: ReadonlyArray<string> | TypedTagIntersection<unknown>,
): e is TypedTagIntersection<unknown> => {
  return !Array.isArray(e)
}

export type PondOptions = {
  hbHistDelay?: number
  currentPsnHistoryDelay?: number
  updateConnectivityEvery?: Milliseconds

  stateEffectDebounce?: number
}

export type PondInfo = {
  sourceId: SourceId
}

export const makeEventChunk = <E>(source: Source, events: ReadonlyArray<E>): UnstoredEvents => {
  const timestamp = Timestamp.now()
  const { semantics, name } = source
  return events.map(payload => ({
    semantics,
    name,
    tags: [],
    timestamp,
    payload,
  }))
}

const omitObservable = <S>(
  callback: (newState: S) => void,
  states: Observable<StateWithProvenance<S>>,
): CancelSubscription => {
  const sub = states.map(x => x.state).subscribe(callback)
  return sub.unsubscribe.bind(sub)
}

const pendingEmission = (o: Observable<void>): PendingEmission => ({
  subscribe: o.subscribe.bind(o),
  toPromise: () => o.toPromise(),
})

type ActiveFish<S> = {
  readonly states: Observable<StateWithProvenance<S>>
  commandPipeline?: CommandPipeline<S, EmissionRequest<any>>
}

export type Pond = {
  /* EMISSION */

  /**
   * Emit a single event directly.
   *
   * @param tags    Tags to attach to the event.
   * @param payload The event payload.
   * @returns       A `PendingEmission` object that can be used to register
   *                callbacks with the emission’s completion.
   */
  emit<E>(tags: string[] | TypedTagIntersection<E>, payload: E): PendingEmission

  /**
   * Emit a number of events at once.
   *
   * @param emit    The events to be emitted, expressed as objects containing `tags` and `payload`.
   * @returns       A `PendingEmission` object that can be used to register
   *                callbacks with the emission’s completion.
   */
  emitMany(...emit: ReadonlyArray<Emit<any>>): PendingEmission

  /* AGGREGATION */

  /**
   * Fold events into state. Caching is done based on the `cacheKey` inside the `fish`.
   *
   * @param fish    Complete aggregation information.
   * @param callback     Function that will be called whenever a new state becomes available.
   * @returns            A function that can be called in order to cancel the aggregation.
   */
  observe<S, E>(fish: Fish<S, E>, callback: (newState: S) => void): CancelSubscription

  /* CONDITIONAL EMISSION (COMMANDS) */

  /**
   * Run StateEffects against the current **locally known** State of the `fish`.
   * The Effect is able to consider that State and create Events from it.
   * Every Effect will see the Events of all previous Effects *on this fish* applied already!
   *
   * In regards to other nodes, there are no serialisation guarantees.
   *
   * @typeParam S                State of the Fish, input value to the effect.
   * @typeParam EWrite           Payload type(s) to be returned by the effect.
   *
   * @param fish         Complete aggregation information.
   * @param effect       A function to turn State into an array of Events. The array may be empty, in order to emit 0 Events.
   * @returns            A `PendingEmission` object that can be used to register callbacks with the effect’s completion.
   */
  run<S, EWrite>(fish: Fish<S, any>, effect: StateEffect<S, EWrite>): PendingEmission

  /**
   * Curried version of `runStateEffect`.
   *
   * @typeParam S                State of the Fish, input value to the effect.
   * @typeParam EWrite           Payload type(s) to be returned by the effect.
   *
   * @param fish         Complete aggregation information.
   * @param effect       A function to turn State into an array of Events. The array may be empty, in order to emit 0 Events.
   * @returns            A `PendingEmission` object that can be used to register callbacks with the effect’s completion.
   */
  runC<S, EWrite>(fish: Fish<S, any>): (effect: StateEffect<S, EWrite>) => PendingEmission

  /**
   * Install a StateEffect that will be applied automatically whenever the `agg`’s State has changed.
   * Every application will see the previous one’s resulting Events applied to the State already, if applicable;
   * but any number of intermediate States may have been skipped between two applications.
   *
   * The effect can be uninstalled by calling the returned `CancelSubscription`.
   *
   * @typeParam S        State of the Fish, input value to the effect.
   * @typeParam EWrite   Payload type(s) to be returned by the effect.
   *
   * @param fish         Complete aggregation information.
   * @param effect       A function to turn State into an array of Events. The array may be empty, in order to emit 0 Events.
   * @param autoCancel   Condition on which the automatic effect will be cancelled -- State on which `autoCancel` returns `true`
   *                     will be the first State the effect is *not* applied to anymore.
   * @returns            A `CancelSubscription` object that can be used to cancel the automatic effect.
   */
  keepRunning<S, EWrite>(
    fish: Fish<S, any>,
    effect: StateEffect<S, EWrite>,
    autoCancel?: (state: S) => boolean,
  ): CancelSubscription

  /*
   * HOUSE KEEPING FUNCTIONS
   */

  /**
   * Dispose subscription to IpfsStore
   * Store subscription needs to be unsubscribed for HMR
   */
  dispose(): Promise<void>

  /**
   * Information about the current pond
   */
  info(): PondInfo

  /**
   * Obtain an observable state of the pond.
   */
  getPondState(): Observable<PondState>

  /**
   * Obtain an observable describing connectivity status of this node.
   */
  getNodeConnectivity(...specialSources: ReadonlyArray<SourceId>): Observable<ConnectivityStatus>

  /**
   * Obtain an observable that completes when we are mostly in sync with the swarm.
   * It is recommended to wait for this on application startup, before interacting with any fish,
   * i.e. `await pond.waitForSwarmSync().toPromise()`. The intermediate states emitted
   * by the Observable can be used to display render a progress bar, for example.
   */
  waitForSwarmSync(config?: WaitForSwarmConfig): Observable<SplashState>
}

export class Pond2Impl implements Pond {
  readonly hydrateV2: <S, E>(
    subscriptionSet: SubscriptionSet,
    initialState: S,
    onEvent: (state: S, event: E, metadata: Metadata) => S,
    fishId: FishId,
    enableLocalSnapshots: boolean,
    isReset?: IsReset<E>,
  ) => Observable<StateWithProvenance<S>>

  activeFishes: {
    [fishId: string]: ActiveFish<any>
  } = {}

  constructor(
    private readonly eventStore: EventStore,
    private readonly snapshotStore: SnapshotStore,
    private readonly pondStateTracker: PondStateTracker,
    private readonly monitoring: Monitoring,
    private readonly opts: PondOptions,
  ) {
    this.hydrateV2 = FishJar.hydrateV2(this.eventStore, this.snapshotStore, this.pondStateTracker)
  }

  getPondState = (): Observable<PondState> => this.pondStateTracker.observe()

  getNodeConnectivity = (
    ...specialSources: ReadonlyArray<SourceId>
  ): Observable<ConnectivityStatus> =>
    this.eventStore.connectivityStatus(
      specialSources,
      this.opts.hbHistDelay || 1e12,
      this.opts.updateConnectivityEvery || Milliseconds.of(10_000),
      this.opts.currentPsnHistoryDelay || 6,
    )

  waitForSwarmSync = (config?: WaitForSwarmConfig): Observable<SplashState> =>
    SplashState.of(this.eventStore, config || {})

  info = () => {
    return {
      sourceId: this.eventStore.sourceId,
    }
  }

  dispose = async () => {
    this.monitoring.dispose()
    // TODO: Implement cleanup of active fishs
  }

  /* POND V2 FUNCTIONS */
  private emitTagged0 = <E>(emit: ReadonlyArray<Emit<E>>): Observable<Events> => {
    const events = emit.map(({ tags, payload }) => {
      const timestamp = Timestamp.now()

      const event = {
        semantics: Semantics.none,
        name: FishName.none,
        tags: isTyped(tags) ? tags.raw().tags : tags,
        timestamp,
        payload,
      }

      return event
    })

    return this.eventStore.persistEvents(events)
  }

  emit = <E>(tags: string[] | TypedTagIntersection<E>, payload: E): PendingEmission => {
    return this.emitMany({ tags, payload })
  }

  emitMany = (...emissions: ReadonlyArray<Emit<any>>): PendingEmission => {
    // `shareReplay` so that every piece of user code calling `subscribe`
    // on the return value will actually be executed
    const o = this.emitTagged0(emissions)
      .mapTo(void 0)
      .shareReplay(1)

    // `o` is already (probably) hot, but we subscribe just in case.
    o.subscribe()

    return pendingEmission(o)
  }

  private getCachedOrInitialize = <S, E>(
    subscriptionSet: SubscriptionSet,
    initialState: S,
    onEvent: Reduce<S, E>,
    fishId: FishId,
    isReset?: IsReset<E>,
  ): ActiveFish<S> => {
    const key = FishId.canonical(fishId)
    const existing = this.activeFishes[key]
    if (existing !== undefined) {
      return {
        ...existing,
        states: existing.states.observeOn(Scheduler.queue),
      }
    }

    const stateSubject = this.hydrateV2(
      subscriptionSet,
      initialState,
      onEvent,
      fishId,
      true,
      isReset,
    ).shareReplay(1)

    const a = {
      states: stateSubject,
    }
    this.activeFishes[key] = a
    return a
  }

  private observeTagBased0 = <S, E>(acc: Fish<S, E>): ActiveFish<S> => {
    const subscriptionSet: SubscriptionSet = {
      type: 'tags',
      subscriptions: toWireFormat(acc.where),
    }

    return this.getCachedOrInitialize(
      subscriptionSet,
      acc.initialState,
      acc.onEvent,
      acc.fishId,
      acc.isReset,
    )
  }

  observe = <S, E>(acc: Fish<S, E>, callback: (newState: S) => void): CancelSubscription => {
    if (acc.deserializeState) {
      throw new Error('custom deser not yet supported')
    }

    return omitObservable(callback, this.observeTagBased0<S, E>(acc).states)
  }

  // Get a (cached) Handle to run StateEffects against. Every Effect will see the previous one applied to the State.
  runC = <S, EWrite, ReadBack = false>(
    agg: Fish<S, ReadBack extends true ? EWrite : any>,
  ): ((effect: StateEffect<S, EWrite>) => PendingEmission) => {
    const cached = this.observeTagBased0(agg)
    const handleInternal = this.getOrCreateCommandHandle0(agg, cached)

    return effect => pendingEmission(handleInternal(effect))
  }

  private v2CommandHandler = (emit: EmissionRequest<unknown>) => {
    return Observable.from(Promise.resolve(emit)).mergeMap(x => this.emitTagged0(x))
  }

  private getOrCreateCommandHandle0 = <S, EWrite, ReadBack = false>(
    agg: Fish<S, ReadBack extends true ? EWrite : any>,
    cached: ActiveFish<S>,
  ): ((effect: StateEffect<S, EWrite>) => Observable<void>) => {
    const handler = this.v2CommandHandler

    const subscriptionSet: SubscriptionSet = {
      type: 'tags',
      subscriptions: toWireFormat(agg.where),
    }

    const commandPipeline =
      cached.commandPipeline ||
      FishJar.commandPipeline<S, EmissionRequest<any>>(
        this.pondStateTracker,
        this.eventStore.sourceId,
        agg.fishId.entityType || Semantics.none,
        agg.fishId.name,
        handler,
        cached.states,
        subscriptionsToEventPredicate(subscriptionSet),
      )
    cached.commandPipeline = commandPipeline

    return effect => {
      const o = new Observable<void>(x =>
        commandPipeline.subject.next({
          type: 'command',
          command: effect,
          onComplete: () => {
            x.next()
            x.complete()
          },
          onError: (err: any) => x.error(err),
        }),
      )
        .shareReplay(1)
        // Subscribing on Scheduler.queue is not strictly required, but helps with dampening feedback loops
        .subscribeOn(Scheduler.queue)

      // We just subscribe to guarantee effect application;
      // user is responsible for handling errors on the returned object if desired.
      o.catch(() => Observable.empty()).subscribe()

      return o
    }
  }

  run = <S, EWrite, ReadBack = false>(
    agg: Fish<S, ReadBack extends true ? EWrite : any>,
    effect: StateEffect<S, EWrite>,
  ): PendingEmission => {
    const handle = this.runC(agg)
    return handle(effect)
  }

  keepRunning = <S, EWrite, ReadBack = false>(
    agg: Fish<S, ReadBack extends true ? EWrite : any>,
    effect: StateEffect<S, EWrite>,
    autoCancel?: (state: S) => boolean,
  ): CancelSubscription => {
    // We use this state `cancelled` to stop effects "asap" when user code calls the cancellation function.
    // Otherwise it might happen that we have already queued the next effect and run longer than desired.
    let cancelled = false

    const wrappedEffect = (state: S) => (cancelled ? [] : effect(state))

    const cached = this.observeTagBased0(agg)
    const states = cached.states
    const handleInternal = this.getOrCreateCommandHandle0(agg, cached)

    const tw = autoCancel
      ? (state: S) => {
          if (cancelled) {
            return false
          } else if (autoCancel(state)) {
            cancelled = true
            return false
          }

          return true
        }
      : () => !cancelled

    states
      .map(swp => swp.state)
      .takeWhile(tw)
      .debounceTime(0)
      // We could also just use `do` instead of `mergeMap` (using the public API),
      // for no real loss, but no gain either.
      .mergeMap(() => handleInternal(wrappedEffect))
      .subscribe()

    return () => (cancelled = true)
  }
}

/**
 * All services needed by the pond
 */
type Services = Readonly<{
  eventStore: EventStore
  snapshotStore: SnapshotStore
  commandInterface: CommandInterface
}>

const mockSetup = (): Services => {
  const eventStore = EventStore.mock()
  const snapshotStore = SnapshotStore.inMem()
  const commandInterface = CommandInterface.mock()
  return { eventStore, snapshotStore, commandInterface }
}

const createServices = async (multiplexer: MultiplexedWebsocket): Promise<Services> => {
  const sourceId = await getSourceId(multiplexer)
  const eventStore = EventStore.ws(multiplexer, sourceId)
  const snapshotStore = SnapshotStore.ws(multiplexer)
  const commandInterface = CommandInterface.ws(multiplexer, sourceId)
  return { eventStore, snapshotStore, commandInterface }
}

const mkPond = async (multiplexer: MultiplexedWebsocket, opts: PondOptions = {}): Promise<Pond> => {
  const services = await createServices(multiplexer || mkMultiplexer())
  return pondFromServices(services, opts)
}

const mkMockPond = async (opts?: PondOptions): Promise<Pond> => {
  const opts1: PondOptions = opts || {}
  const services = mockSetup()
  return pondFromServices(services, opts1)
}

export type TestPond2 = Pond & {
  directlyPushEvents: (events: Events) => void
  eventStore: TestEventStore
}
const mkTestPond = async (opts?: PondOptions): Promise<TestPond2> => {
  const opts1: PondOptions = opts || {}
  const eventStore = EventStore.test(SourceId.of('TEST'))
  const snapshotStore = SnapshotStore.inMem()
  const commandInterface = CommandInterface.mock()
  return {
    ...pondFromServices({ eventStore, snapshotStore, commandInterface }, opts1),
    directlyPushEvents: eventStore.directlyPushEvents,
    eventStore,
  }
}
const pondFromServices = (services: Services, opts: PondOptions): Pond => {
  const { eventStore, snapshotStore, commandInterface } = services

  const monitoring = Monitoring.of(commandInterface, 10000)

  log.pond.debug('start pond with SourceID %s from store', eventStore.sourceId)

  const pondStateTracker = mkPondStateTracker(log.pond)
  const pond: Pond2Impl = new Pond2Impl(
    eventStore,
    snapshotStore,
    pondStateTracker,
    monitoring,
    opts,
  )

  return pond
}

export const Pond = {
  default: async (): Promise<Pond> => Pond.of(mkMultiplexer()),
  of: mkPond,
  mock: mkMockPond,
  test: mkTestPond,
}