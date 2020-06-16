import { Observable } from 'rxjs'
import { Aggregate, Pond, PondV2, TagQuery } from '.'

const emitTestEvents = async (pond: PondV2) =>
  pond
    .emitEvents(
      { tags: ['t0', 't1', 't2'], payload: 'hello' },
      { tags: ['t0', 't1', 't2'], payload: 'world' },
      { tags: ['t1'], payload: 't1 only' },
      { tags: ['t2'], payload: 't2 only' },
    )
    .toPromise()

const assertStateAndDispose = async <S>(states: Observable<S>, expected: S, pond: Pond) => {
  const res = states
    .debounceTime(5)
    .take(1)
    .toPromise()

  await expect(res).resolves.toEqual(expected)

  await pond.dispose()
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const aggregateAsObservable = <S>(pond: PondV2, agg: Aggregate<S, any>): Observable<S> =>
  new Observable(x => {
    pond.aggregate(agg, s => x.next(s))
  })

describe('application of commands in the pond', () => {
  const expectAggregationToYield = async (subscriptions: TagQuery, expectedResult: string[]) => {
    const pond = await Pond.test()

    const aggregate = Aggregate.eventsDescending<string>(subscriptions)

    const res = aggregateAsObservable(pond, aggregate)

    await emitTestEvents(pond)

    await assertStateAndDispose(res, expectedResult, pond)
  }

  it('should aggregate based on tags intersection', async () =>
    expectAggregationToYield(TagQuery.requireAll('t0', 't1'), ['world', 'hello']))

  it('should aggregate based on tags union', async () =>
    expectAggregationToYield(TagQuery.union('t0', 't1'), ['t1 only', 'world', 'hello']))

  it('should aggregate based on single tag', async () =>
    expectAggregationToYield(TagQuery.requireEvery('t2'), ['t2 only', 'world', 'hello']))

  it('should aggregate everything', async () =>
    // Empty intersection matches everything
    expectAggregationToYield(TagQuery.requireEvery(), ['t2 only', 't1 only', 'world', 'hello']))

  it('should aggregate nothing', async () =>
    // Empty union means not a single subscription
    expectAggregationToYield(TagQuery.requireSome(), []))

  describe('caching', () => {
    type Event = string
    type State = ReadonlyArray<Event>

    const mkAggregate = (subscriptions: TagQuery, cacheKey = { name: 'test-entity' }) => ({
      subscriptions,

      initialState: [],

      onEvent: (state: State, event: Event) => [event, ...state],

      cacheKey,
    })

    it('should cache based on key', async () => {
      const pond = await Pond.test()

      const aggregate0 = mkAggregate(TagQuery.union('t1'))

      const unsubscribe0 = pond.aggregate<State, Event>(aggregate0, _s => {
        /* swallow */
      })

      await emitTestEvents(pond)

      // use same name, but different subscriptions, to assert that cached aggregate is re-used
      const aggregate1 = mkAggregate(TagQuery.union('t99'))

      const res = aggregateAsObservable(pond, aggregate1)

      unsubscribe0()
      await assertStateAndDispose(res, ['t1 only', 'world', 'hello'], pond)
    })

    it('should cache based on key, across unsubscribe calls', async () => {
      const pond = await Pond.test()

      const aggregate = mkAggregate(TagQuery.union('t1'))

      const unsubscribe0 = pond.aggregate<State, Event>(aggregate, _s => {
        /* swallow */
      })

      await emitTestEvents(pond)
      unsubscribe0()
      await Observable.timer(500).toPromise()

      // should immediately pick up the existing aggregation's pipeline
      const res = new Promise((resolve, _reject) =>
        pond.aggregate(aggregate, state => resolve(state)),
      )

      await expect(res).resolves.toEqual(['t1 only', 'world', 'hello'])

      pond.dispose()
    })

    it('should permit different aggregations in parallel', async () => {
      const pond = await Pond.test()

      const aggregate0 = mkAggregate(TagQuery.union('t0'))

      const unsubscribe0 = pond.aggregate<State, Event>(aggregate0, _s => {
        /* swallow */
      })

      await emitTestEvents(pond)

      // use a different cache key to start another aggregation
      const aggregate1 = mkAggregate(TagQuery.union('t1'), { name: 'different-name' })

      const res = aggregateAsObservable(pond, aggregate1)

      unsubscribe0()
      await assertStateAndDispose(res, ['t1 only', 'world', 'hello'], pond)
    })
  })
})
