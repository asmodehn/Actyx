import { Observable, Scheduler } from 'rxjs'
import { marbles } from 'rxjs-marbles'
import { AdtTransform, concatHot, takeWhileInclusive } from './observable'

describe('concatHot', () => {
  const values = { a: 1, b: 2, c: 3, d: 4, e: 5, f: 6, g: 7 }
  it(
    'should concatenate hot observables without losing elements',
    marbles(m => {
      const a = m.hot('^-a-b-|', values)
      const b = m.hot('^-c-d-e-----f-g', values)
      const e = m.hot('^-a-b-(cde)-f-g', values)
      const c = concatHot(a, b)
      m.expect(c).toBeObservable(e)
    }),
  )

  it(
    'should unsubscribe from the second arg when the first arg fails',
    marbles(m => {
      const a = m.hot('^-a-b-#', values)
      const b = m.hot('^-c-d-e-----f-g', values)
      const e = m.hot('^-a-b-#', values)
      const bsubs = '^-----!'

      const c = concatHot(a, b)
      m.expect(c).toBeObservable(e)
      m.expect(b).toHaveSubscriptions(bsubs)
    }),
  )
})

type A = { readonly type: 'a' }
type B = { readonly type: 'b' }
type ADT = A | B

describe('AdtTransform.combine', () => {
  it('should allow transforming adts', async () => {
    const a = (x: Observable<A>): Observable<string> => x.map(() => 'a')
    const b = (x: Observable<B>): Observable<string> => x.map(() => 'b')
    const transform = AdtTransform.combine<ADT, string>({
      a,
      b,
    })
    const ax: A = { type: 'a' }
    const bx: B = { type: 'b' }
    const t = Observable.from<ADT>([ax, bx, ax, bx])
    const result = await t
      .pipe(transform)
      .toArray()
      .toPromise()
    expect(result).toEqual(['a', 'b', 'a', 'b'])
  })
})

describe('takeWhileInclusive', () => {
  it('should takeWhile predicate and then emit one more', async () => {
    const result = await Observable.from([1, 2, 3, 4, 5, 6, 6, 7])
      .pipe(takeWhileInclusive(e => e < 6))
      .toArray()
      .toPromise()
    expect(result).toEqual([1, 2, 3, 4, 5, 6]) // just one 6, the one on which the predicate has fired
    const result2 = await Observable.from([1, 2, 3, 4, 5, 6, 6, 7, 8, 8, 9])
      .pipe(takeWhileInclusive(e => e < 8))
      .toArray()
      .toPromise()
    expect(result2).toEqual([1, 2, 3, 4, 5, 6, 6, 7, 8]) // just one 8
  })

  it('should unsubscribe when done with stuff', async () => {
    let count = 0
    const result = await Observable.from([1, 2, 3, 4, 5, 6, 7, 8, 9])
      .observeOn(Scheduler.queue)
      .do(_ => ++count)
      .takeWhile(e => e < 6)
      .toArray()
      .toPromise()

    let count2 = 0
    const result2 = await Observable.from([1, 2, 3, 4, 5, 6, 7, 8, 9])
      .observeOn(Scheduler.queue)
      .do(_ => ++count2)
      .pipe(takeWhileInclusive(e => e < 5))
      .toArray()
      .toPromise()
    expect(result).toEqual([1, 2, 3, 4, 5])
    expect(count).toEqual(6)
    expect(result2).toEqual([1, 2, 3, 4, 5])
    expect(count2).toEqual(5)
  })

  it(
    'should unsubscribe from upstream after it gets the result 2',
    marbles(m => {
      const values = { a: 1, b: 2, c: 3, d: 4, e: 5, f: 6, g: 7 }

      const a = m.hot('^-a-b----c-d-e-f-g-|', values)
      const bsubs = '^---!'
      const e = m.hot('^-a-(b|)', values)

      const c = a.pipe(takeWhileInclusive(z => z < 2))
      m.expect(c).toBeObservable(e)
      m.expect(a).toHaveSubscriptions(bsubs)
    }),
  )

  it('should handle errors', async () => {
    let count = 0
    const result2 = await Observable.from([1, 2, 3, 4])
      .concat(Observable.throw('BOOM!'))
      .concat(Observable.from([6, 7, 8, 9]))
      .do(_ => ++count)
      .pipe(takeWhileInclusive(e => e < 10))
      .catch(e => Observable.of(e))
      .toArray()
      .toPromise()
    expect(result2).toEqual([1, 2, 3, 4, 'BOOM!'])
    expect(count).toEqual(4)
  })
})
