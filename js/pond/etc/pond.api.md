## API Report File for "@actyx/pond"

> Do not edit this file. It is a report generated by [API Extractor](https://api-extractor.com/).

```ts

import * as immutable from 'immutable';
import * as t from 'io-ts';

// @public
export type AddEmission<EWrite> = <E extends EWrite>(tags: Tags<E>, event: E) => void;

// @public
export const allEvents: Tags<unknown>;

// @public
export type CancelSubscription = () => void;

// @public
export const ConnectivityStatus: t.UnionC<[t.ReadonlyC<t.TypeC<{
    status: t.LiteralC<ConnectivityStatusType.FullyConnected>;
    inCurrentStatusForMs: t.NumberC;
}>>, t.ReadonlyC<t.TypeC<{
    status: t.LiteralC<ConnectivityStatusType.PartiallyConnected>;
    inCurrentStatusForMs: t.NumberC;
    specialsDisconnected: t.ReadonlyArrayC<t.StringC>;
    swarmConnectivityLevel: t.NumberC;
    eventsToRead: t.NumberC;
    eventsToSend: t.NumberC;
}>>, t.ReadonlyC<t.TypeC<{
    status: t.LiteralC<ConnectivityStatusType.NotConnected>;
    inCurrentStatusForMs: t.NumberC;
    eventsToRead: t.NumberC;
    eventsToSend: t.NumberC;
}>>]>;

// @public
export type ConnectivityStatus = t.TypeOf<typeof ConnectivityStatus>;

// @public
export enum ConnectivityStatusType {
    // (undocumented)
    FullyConnected = "FullyConnected",
    // (undocumented)
    NotConnected = "NotConnected",
    // (undocumented)
    PartiallyConnected = "PartiallyConnected"
}

// @public
export type Counters = Readonly<CountersMut>;

// @public
export type CountersMut = {
    own: number;
    swarm: number;
    both: number;
};

// @public
export const enableAllLoggersExcept: (excludeModules: string[]) => void;

// @public
export type Fish<S, E> = Readonly<{
    where: Where<E>;
    initialState: S;
    onEvent: Reduce<S, E>;
    fishId: FishId;
    isReset?: IsReset<E>;
    deserializeState?: (jsonState: unknown) => S;
}>;

// @public
export const Fish: {
    latestEvent: <E>(where: Where<E>) => Readonly<{
        where: Where<E>;
        initialState: E | undefined;
        onEvent: Reduce<E | undefined, E>;
        fishId: FishId;
        isReset?: IsReset<E> | undefined;
        deserializeState?: ((jsonState: unknown) => E | undefined) | undefined;
    }>;
    eventsDescending: <E_1>(where: Where<E_1>, capacity?: number) => Readonly<{
        where: Where<E_1>;
        initialState: E_1[];
        onEvent: Reduce<E_1[], E_1>;
        fishId: FishId;
        isReset?: IsReset<E_1> | undefined;
        deserializeState?: ((jsonState: unknown) => E_1[]) | undefined;
    }>;
    eventsAscending: <E_2>(where: Where<E_2>, capacity?: number) => Readonly<{
        where: Where<E_2>;
        initialState: E_2[];
        onEvent: Reduce<E_2[], E_2>;
        fishId: FishId;
        isReset?: IsReset<E_2> | undefined;
        deserializeState?: ((jsonState: unknown) => E_2[]) | undefined;
    }>;
};

// @public
export type FishId = {
    entityType: string;
    name: string;
    version: number;
};

// @public
export const FishId: {
    of: (entityType: string, name: string, version: number) => {
        entityType: string;
        name: string;
        version: number;
    };
    canonical: (v: FishId) => string;
};

// @public
export type FishProcessInfo = {
    numBeingProcessed: number;
    fish: {
        [semantics: string]: true | undefined;
    };
};

// @public
export type FullWaitForSwarmConfig = Readonly<{
    enabled: boolean;
    waitForSwarmMs: number;
    minSources: number;
    waitForSyncMs?: number;
    allowSkip: boolean;
}>;

// @public
export type GetNodeConnectivityParams = Readonly<{
    callback: (newState: ConnectivityStatus) => void;
    specialSources?: ReadonlyArray<SourceId>;
}>;

// @public
export const isBoolean: (x: any) => x is boolean;

// @public
export const isNumber: (x: any) => x is number;

// @public
export type IsReset<E> = (event: E, metadata: Metadata) => boolean;

// @public
export const isString: (x: any) => x is string;

// @public
export type Lamport = number;

// @public (undocumented)
export const Lamport: {
    of: (value: number) => Lamport;
    zero: number;
    FromNumber: t.Type<number, number, unknown>;
};

// @public
export type LogFunction = ((first: any, ...rest: any[]) => void);

// @public
export interface Logger extends LogFunction {
    // (undocumented)
    readonly enabled: boolean;
    // (undocumented)
    readonly namespace: string;
}

// @public
export type Loggers = {
    error: Logger;
    warn: Logger;
    debug: Logger;
    info: Logger;
};

// @public
export const Loggers: {
    of: (topic: string) => Loggers;
};

// @public
export type Metadata = Readonly<{
    isLocalEvent: boolean;
    tags: ReadonlyArray<string>;
    timestampMicros: Timestamp;
    timestampAsDate: () => Date;
    lamport: Lamport;
    eventId: string;
}>;

// @public
export type Milliseconds = number;

// @public
export const Milliseconds: {
    of: (time: number) => Milliseconds;
    fromDate: (date: Date) => Milliseconds;
    zero: number;
    now: (now?: number | undefined) => Milliseconds;
    toSeconds: (value: Milliseconds) => number;
    toTimestamp: (value: Milliseconds) => Timestamp;
    fromSeconds: (value: number) => number;
    fromMinutes: (value: number) => number;
    fromAny: (value: number) => Milliseconds;
    FromNumber: t.Type<number, number, unknown>;
};

// @public
export type NodeInfoEntry = Readonly<{
    own?: number;
    swarm?: number;
}>;

// @public
export const noEvents: Where<never>;

// @public
export type PendingEmission = {
    subscribe: (whenEmitted: () => void) => void;
    toPromise: () => Promise<void>;
};

// @public
export type Pond = {
    emit<E>(tags: Tags<E>, event: E): PendingEmission;
    observe<S, E>(fish: Fish<S, E>, callback: (newState: S) => void): CancelSubscription;
    run<S, EWrite>(fish: Fish<S, any>, fn: StateEffect<S, EWrite>): PendingEmission;
    keepRunning<S, EWrite>(fish: Fish<S, any>, fn: StateEffect<S, EWrite>, autoCancel?: (state: S) => boolean): CancelSubscription;
    dispose(): void;
    info(): PondInfo;
    getPondState(callback: (newState: PondState) => void): CancelSubscription;
    getNodeConnectivity(params: GetNodeConnectivityParams): CancelSubscription;
    waitForSwarmSync(params: WaitForSwarmSyncParams): void;
};

// @public
export const Pond: {
    default: () => Promise<Pond>;
    of: (connectionOpts: Partial<WsStoreConfig>, opts: PondOptions) => Promise<Pond>;
    mock: (opts?: PondOptions | undefined) => Promise<Pond>;
    test: (opts?: PondOptions | undefined) => Promise<TestPond>;
};

// @public
export type PondInfo = {
    sourceId: SourceId;
};

// @public
export type PondOptions = {
    hbHistDelay?: number;
    currentPsnHistoryDelay?: number;
    updateConnectivityEvery?: Milliseconds;
    stateEffectDebounce?: number;
};

// @public
export type PondState = {
    hydration: FishProcessInfo;
    commands: FishProcessInfo;
    eventsFromOtherSources: FishProcessInfo;
};

// @public
export const PondState: {
    isHydrating: (state: PondState) => boolean;
    isProcessingCommands: (state: PondState) => boolean;
    isProcessingEventsFromOtherSources: (state: PondState) => boolean;
    isBusy: (state: PondState) => boolean;
};

// @public
export type Progress = Readonly<{
    min: number;
    current: number;
    max: number;
}>;

// @public
export type Reduce<S, E> = (state: S, event: E, metadata: Metadata) => S;

// @public
export type SourceId = string;

// @public
export const SourceId: {
    of: (text: string) => SourceId;
    random: (digits?: number | undefined) => string;
    FromString: t.Type<string, string, unknown>;
};

// @public
export type SplashState = SplashStateDiscovery | SplashStateSync;

// @public
export type SplashStateDiscovery = Readonly<{
    mode: 'discovery';
    current: SwarmSummary;
    skip?: () => void;
}>;

// @public
export type SplashStateSync = Readonly<{
    mode: 'sync';
    reference: SwarmSummary;
    progress: SyncProgress;
    current: SwarmSummary;
    skip?: () => void;
}>;

// @public
export type StateEffect<S, EWrite> = (state: S, enqueue: AddEmission<EWrite>) => void | Promise<void>;

// @public
export type StoreConfig = Readonly<{
    monitoringMeta?: object;
    metaMs: number;
    runStatsPeriodMs: number;
}>;

// @public
export type StoreConnectionClosedHook = () => void;

// @public
export type SwarmInfo = Readonly<{
    nodes: immutable.Map<string, NodeInfoEntry>;
}>;

// @public
export type SwarmSummary = Readonly<{
    info: SwarmInfo;
    sources: Counters;
    events: Counters;
}>;

// @public
export const SwarmSummary: {
    empty: Readonly<{
        info: SwarmInfo;
        sources: Counters;
        events: Counters;
    }>;
    fromSwarmInfo: (info: SwarmInfo) => SwarmSummary;
};

// @public
export type SyncProgress = Readonly<{
    sources: Progress;
    events: Progress;
}>;

// @public
export interface Tag<E> extends Tags<E> {
    // (undocumented)
    readonly rawTag: string;
    withId(name: string): Tags<E>;
}

// @public
export const Tag: <E>(rawTag: string) => Tag<E>;

// @public
export interface Tags<E> extends Where<E> {
    and<E1>(tag: Tags<E1>): Tags<Extract<E1, E>>;
    and(tag: string): Tags<E>;
    local(): Tags<E>;
}

// @public
export const Tags: <E>(...requiredTags: string[]) => Tags<E>;

// @public
export type TestEvent = {
    psn: number;
    sourceId: string;
    timestamp: Timestamp;
    lamport: Lamport;
    tags: ReadonlyArray<string>;
    payload: unknown;
};

// @public
export type TestPond = Pond & {
    directlyPushEvents: (events: TestEvent[]) => void;
};

// @public
export type Timestamp = number;

// @public
export const Timestamp: {
    of: (time: number) => Timestamp;
    zero: number;
    maxSafe: number;
    now: (now?: number | undefined) => number;
    format: (timestamp: Timestamp) => string;
    toSeconds: (value: Timestamp) => number;
    toMilliseconds: (value: Timestamp) => Milliseconds;
    toDate: (value: Timestamp) => Date;
    fromDate: (date: Date) => Timestamp;
    fromDays: (value: number) => number;
    fromSeconds: (value: number) => number;
    fromMilliseconds: (value: number) => number;
    min: (...values: Timestamp[]) => number;
    max: (values: Timestamp[]) => number;
    FromNumber: t.Type<number, number, unknown>;
};

// @public
export const unreachable: (x?: never) => never;

// @public
export function unreachableOrElse<T>(_: never, t: T): T;

// @public
export type WaitForSwarmConfig = Partial<FullWaitForSwarmConfig>;

// @public
export const WaitForSwarmConfig: {
    defaults: Readonly<{
        enabled: boolean;
        waitForSwarmMs: number;
        minSources: number;
        waitForSyncMs?: number | undefined;
        allowSkip: boolean;
    }>;
};

// @public
export type WaitForSwarmSyncParams = WaitForSwarmConfig & Readonly<{
    onSyncComplete: () => void;
    onProgress?: (newState: SplashState) => void;
}>;

// @public
export interface Where<E> {
    readonly _dataType?: E;
    or<E1>(tag: Where<E1>): Where<E1 | E>;
    toString(): string;
}

// @public
export type WsStoreConfig = Readonly<{
    url: string;
    protocol?: string;
    onStoreConnectionClosed?: StoreConnectionClosedHook;
    reconnectTimeout?: number;
}>;


// (No @packageDocumentation comment for this package)

```