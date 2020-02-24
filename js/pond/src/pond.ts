/* eslint-disable @typescript-eslint/no-explicit-any */

import * as R from 'ramda'
import { Observable, ReplaySubject, Scheduler, Subject } from 'rxjs'
import { CommandInterface } from './commandInterface'
import { EventStore } from './eventstore'
import { MultiplexedWebsocket } from './eventstore/multiplexedWebsocket'
import { TestEventStore } from './eventstore/testEventStore'
import { ConnectivityStatus, Event, Events, UnstoredEvents } from './eventstore/types'
import { mkMultiplexer } from './eventstore/utils'
import { getSourceId } from './eventstore/websocketEventStore'
import { CommandExecutor } from './executors/commandExecutor'
import { FishJar } from './fishJar'
import log from './loggers'
import { mkPondStateTracker, PondState, PondStateTracker } from './pond-state'
import { SnapshotStore } from './snapshotStore'
import { Config as WaitForSwarmConfig, SplashState } from './splashState'
import { Monitoring } from './store/monitoring'
import { EnvelopeFromStore } from './store/util'
import {
  Envelope,
  FishName,
  FishType,
  FishTypeImpl,
  Lamport,
  Milliseconds,
  ObserveMethod,
  Psn,
  SendCommand,
  Source,
  SourceId,
  Timestamp,
} from './types'

type AnyFishJar = FishJar<any, any, any>

export type EventChunkOld<E> = {
  source: Source
  timestamp: Timestamp
  events: ReadonlyArray<E>
}
/**
 * Makes a chunk from a number of envelopes
 *
 * This is exclusively needed in old tests and should be removed once the tests are
 * refactored.
 * @param envelopes A number of envelopes
 */
export const mkEventChunk = <E>(envelopes: ReadonlyArray<Envelope<E>>): EventChunkOld<E> => {
  if (envelopes.length === 0) {
    throw new Error()
  }
  const source = envelopes[0].source

  const timestamp = envelopes[0].timestamp
  const events = envelopes.map(x => x.payload)
  if (envelopes.length > 1) {
    if (!envelopes.every(x => R.equals(x.source, source))) {
      throw new Error('all events must have the same source')
    }
    if (!envelopes.every(x => R.equals(x.timestamp, timestamp))) {
      throw new Error('all events must have the same timestamp')
    }
  }
  return {
    timestamp,
    source,
    events,
  }
}

export type SendToStore = <E>(
  source: Source,
  events: ReadonlyArray<E>,
) => Observable<ReadonlyArray<EnvelopeFromStore<E>>>

/**
 * @deprecated
 * As the PSN is given out by the store, we can't do this conversion anymore.
 * Should be only used for testing purposes.
 */
export function flattenChunk(chunk: UnstoredEvents, sourceId: SourceId): Event[] {
  return chunk.map(event => {
    const result = {
      name: event.name,
      semantics: event.semantics,
      sourceId,
      psn: Psn.of(-1),
      lamport: Lamport.of(0), // deprecated - test use only
      timestamp: event.timestamp,
      payload: event.payload,
    }
    return result
  })
}

export type PondInfo = {
  sourceId: SourceId
}
export type Pond = {
  /**
   * Obtain an observable stream of states from the given fish, waking it up if it is
   * not already actively running within this pond. It is guaranteed that after a
   * change in state there will eventually be a current state object emitted by the
   * returned observable, but not every intermediate state is guaranteed to be emitted.
   */
  observe<C, E, P>(fish: FishType<C, E, P>, name: string): Observable<P>

  /**
   * Send a command to the given fish, ensuring that it is woken up within this pond.
   * Processing a command usually results in the generation of events (which in turn
   * trigger observable state changes) or other effects. The events are emitted and
   * applied first (with running of effects as described for runEvent(), but without
   * dispatching intermediate state updates), then effects
   * are executed, followed finally by dispatching the new state if a change in state
   * did in fact occur.
   *
   * Observable completes when command execution finishes.
   *
   * NOTE that this method returns a lazy observable, i.e. if not consumed then the
   * command is also not sent.
   *
   * We are using two argument lists to help the type inference. The first argument list
   * fully determines C, and the second argument list just has to check for matching.
   */
  feed<C, E, P>(fish: FishType<C, E, P>, name: string): (command: C) => Observable<void>

  /**
   * @deprecated
   * Events as they are generated by the processing of commands (feed), flattened
   */
  _events(): Observable<Envelope<any>>

  /**
   * Commands as they are generated by the processing of commands (feed)
   */
  commands(): Observable<SendCommand<any>>

  /**
   * Dump all internal state of all fish, for debugging purposes.
   */
  dump(): Observable<string>

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

const logPondError = { error: (x: any) => log.pond.error(JSON.stringify(x)) }

export type TimeInjector = (source: Source, events: ReadonlyArray<any>) => Timestamp

export const defaultTimeInjector: TimeInjector = (_source: Source, _events: ReadonlyArray<any>) =>
  Timestamp.now()

export type Tap<T> = (xs: Observable<T>) => Observable<T>
const identity = <T>(x: Observable<T>) => x
export const Tap = {
  none: identity,
}
export type CommandTap = Tap<SendCommand<any>>
export type EventTap = Tap<UnstoredEvents>
export type PondOptions = {
  timeInjector?: TimeInjector
  commandTap?: CommandTap
  eventTap?: EventTap

  hbHistDelay?: number
  currentPsnHistoryDelay?: number
  updateConnectivityEvery?: Milliseconds
}

const defaultPondOptions = {
  timeInjector: defaultTimeInjector,
}

export const makeEventChunk = <E>(
  timeInjector: TimeInjector,
  source: Source,
  events: ReadonlyArray<E>,
): UnstoredEvents => {
  const timestamp = timeInjector(source, events)
  const { semantics, name } = source
  return events.map(payload => ({
    semantics,
    name,
    timestamp,
    payload,
  }))
}

export class PondImpl implements Pond {
  commandsSubject: Subject<SendCommand<any>> = new Subject()
  eventsSubject: Subject<UnstoredEvents> = new Subject()
  timeInjector: TimeInjector
  eventTap: EventTap

  // fish containers
  jars: {
    [semantics: string]: { [name: string]: ReplaySubject<AnyFishJar> }
  } = {}

  // executor for async commands
  commandExecutor: CommandExecutor

  constructor(
    readonly eventStore: EventStore,
    readonly snapshotStore: SnapshotStore,
    readonly pondStateTracker: PondStateTracker,
    readonly monitoring: Monitoring,
    readonly opts: PondOptions,
  ) {
    this.eventTap = opts.eventTap || Tap.none
    this.timeInjector = opts.timeInjector ? opts.timeInjector : defaultPondOptions.timeInjector

    const config = {
      getState: <P>(f: FishType<any, any, P>, name: FishName): Promise<P> => {
        return this.observe(f, name)
          .take(1)
          .toPromise()
      },
      sendCommand: <T>(sc: SendCommand<T>) => {
        this.commandsSubject.next(sc)
      },
    }
    this.commandExecutor = CommandExecutor(config)
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

  commands = () => {
    return this.commandsSubject.asObservable()
  }

  // deprecated, testing use only
  _events = () => {
    return this.eventsSubject.mergeMap(c =>
      flattenChunk(c, this.eventStore.sourceId).map(ev => Event.toEnvelopeFromStore<any>(ev)),
    )
  }

  observe = <C, E, P>(fish: FishType<C, E, P>, name: string): Observable<P> => {
    return this.getOrHydrateJar(FishTypeImpl.downcast(fish), FishName.of(name)).concatMap(
      jar => jar.publicSubject,
    )
  }

  allFishJars = (): Observable<AnyFishJar> => {
    return Observable.of(this.jars)
      .concatMap(f => Observable.from(Object.keys(f)).map(k => f[k]))
      .concatMap(f => Observable.from(Object.keys(f)).map(k => f[k]))
      .concatMap(x => x)
  }

  dump = (): Observable<string> => {
    return this.allFishJars()
      .map(s => s.dump())
      .toArray()
      .map(arr => arr.join('\n'))
  }

  getOrHydrateJar = (
    fish: FishTypeImpl<any, any, any, any>,
    name: FishName,
  ): Observable<AnyFishJar> => {
    const semantics = fish.semantics
    const jarPath = [semantics, name]
    const existingSubject = R.pathOr<undefined, ReplaySubject<AnyFishJar>>(
      undefined,
      jarPath,
      this.jars,
    )

    if (existingSubject !== undefined) {
      return existingSubject.observeOn(Scheduler.queue).take(1)
    }

    const subject = new ReplaySubject<AnyFishJar>(1)
    this.jars = R.assocPath(jarPath, subject, this.jars)
    const storeEvents: SendToStore = (source, events) => {
      const chunk = makeEventChunk(this.timeInjector, source, events)
      return Observable.of(chunk).concatMap(x => {
        this.eventsSubject.next(x)
        return this.eventStore
          .persistEvents(chunk)
          .map(c => c.map(ev => Event.toEnvelopeFromStore(ev)))
      })
    }

    FishJar.hydrate(
      fish,
      name,
      this.eventStore,
      this.snapshotStore,
      storeEvents,
      this.observe as ObserveMethod,
      this.commandExecutor,
      this.pondStateTracker,
    ).subscribe(subject)
    return subject
  }

  feed0 = <C, E, P>(fish: FishType<C, E, P>, name: FishName, command: C): Observable<void> => {
    return this.getOrHydrateJar(FishTypeImpl.downcast(fish), name).mergeMap(
      jar =>
        new Observable<void>(x =>
          jar.enqueueCommand(
            command,
            () => {
              x.next()
              x.complete()
            },
            err => x.error(err),
          ),
        ),
    )
  }

  feed = <C, E, P>(fish: FishType<C, E, P>, name: FishName) => {
    return (command: C) => this.feed0(fish, name, command)
  }

  info = () => {
    return {
      sourceId: this.eventStore.sourceId,
    }
  }

  dispose = () => {
    this.monitoring.dispose()
    return this.allFishJars()
      .do(jar => jar.dispose())
      .defaultIfEmpty(undefined)
      .last()
      .do(() => (this.jars = {}))
      .mapTo(undefined)
      .toPromise()
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

type TestPond = Pond & {
  directlyPushEvents: (events: Events) => void
  eventStore: TestEventStore
}
const mkTestPond = async (opts?: PondOptions): Promise<TestPond> => {
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
  const pond: PondImpl = new PondImpl(eventStore, snapshotStore, pondStateTracker, monitoring, opts)
  // execute commands by calling feed
  pond.commandsSubject
    .pipe(opts.commandTap || Tap.none)
    .mergeMap(s => pond.feed0(s.target.semantics, FishName.of(s.target.name), s.command))
    .subscribe(logPondError)

  return pond
}

export const Pond = {
  default: async (): Promise<Pond> => Pond.of(mkMultiplexer()),
  of: mkPond,
  mock: mkMockPond,
  test: mkTestPond,
}
