/* eslint-disable @typescript-eslint/no-explicit-any */
import { fromNullable } from 'fp-ts/lib/Option'
import { literal, union } from 'io-ts'

/**
 * Base type for a tagged union with type as the tag field
 */

export type Tagged = { readonly type: string }
/**
 * Extract a case from a tagged union based on the (singleton) type of the tag
 */
export type SelectTag<T, Tag> = T extends { type: Tag } ? T : never
/**
 * Type transform that makes a nested type partial, while
 * leaving alone arrays, readonly arrays, functions and scalars
 */
export type DeepPartial<T> = T extends ReadonlyArray<any>
  ? T
  : T extends Function ? T : T extends object ? { [K in keyof T]?: DeepPartial<T[K]> } : T

export const todo: (...args: any[]) => never = () => {
  throw new Error('not implemented yet')
}

export const none: () => never = () => {
  throw new Error('there should have been something')
}

export const unreachable: (x?: never) => never = () => {
  throw new Error('Unreachable code!')
}

export function unreachableOrElse<T>(_: never, t: T): T {
  return t
}
/**
 * Avoids lint false positives like "Expression is always false. (strict-type-predicates)"
 */
export const lookup = <V>(m: { [k: string]: V }, k: string): V | undefined => m[k]

/**
 * Helpers for dealing with "type X = { [ key: K]: Y | undefined }
 */

type RecordKey = string | number | symbol
export type RWPartialRecord<K extends RecordKey, V> = { [P in K]?: V }
export type PartialRecord<K extends RecordKey, V> = Readonly<RWPartialRecord<K, V>>
export type PartialRecord2<K1 extends RecordKey, K2 extends RecordKey, V> = {
  readonly [P in K1]?: { readonly [T in K2]?: V }
}
export type RWKeyValueMap<T> = RWPartialRecord<string, T>
export type KeyValueMap<T> = PartialRecord<string, T>

export const isDefined = <T>(v: T): v is Exclude<T, undefined> => v !== undefined
export const valuesOf = <V>(m: PartialRecord<any, V>): Exclude<V, undefined>[] =>
  Object.values(m).filter(isDefined)

export const keysOf = <K extends RecordKey, V>(m: PartialRecord<K, V>): Extract<K, string>[] =>
  (Object.keys(m) as any) as Extract<K, string>[]

export const entriesOf = <K extends RecordKey, V>(
  obj: PartialRecord<K, V>,
): [Extract<K, string>, V][] =>
  (Object.entries(obj).filter(([, x]) => isDefined(x)) as any) as [Extract<K, string>, V][]

export const entriesOf2 = <K1 extends RecordKey, K2 extends RecordKey, V>(
  m1: PartialRecord2<K1, K2, V>,
): ReadonlyArray<[Extract<K1, string>, Extract<K2, string>, V]> =>
  entriesOf(m1).reduce<ReadonlyArray<[Extract<K1, string>, Extract<K2, string>, V]>>(
    (acc, [k1, m2]) =>
      acc.concat(
        entriesOf<K2, V>(m2).map(
          ([k2, v]): [Extract<K1, string>, Extract<K2, string>, V] => [k1, k2, v],
        ),
      ),
    [],
  )

export const toKeyValueMapF = <T, U>(
  getKey: (v: T) => string,
  map: (v: T) => U,
  xs: ReadonlyArray<T>,
): KeyValueMap<U> =>
  xs.reduce<RWKeyValueMap<U>>((acc, v) => {
    acc[getKey(v)] = map(v)
    return acc
  }, {})

export const toKeyValueMap = <T, K extends keyof T>(key: K, xs: ReadonlyArray<T>): KeyValueMap<T> =>
  xs.reduce((acc: any, v) => {
    acc[v[key]] = v
    return acc
  }, {})

export const tuple = <T extends any[]>(...data: T) => data

export const PartialRecord = {
  get<K extends RecordKey, V>(m: PartialRecord<K, V>, key: K): PartialRecord<K, V>[K] {
    return m[key]
  },
  get2<K1 extends RecordKey, K2 extends RecordKey, V>(
    m: PartialRecord2<K1, K2, V>,
    k1: K1,
    k2: K2,
  ): PartialRecord<K2, V>[K2] | undefined {
    return fromNullable<PartialRecord<K2, V>>(m[k1])
      .mapNullable(m1 => m1[k2])
      .toUndefined()
  },
}

// Not sure if this should go here
// This function will fail horribly if the enum keys are not equal to it's values
export const enumToUnion = <T>(_enum: T) => {
  const [first, second, ...rest] = Object.keys(_enum).map(v =>
    literal<Extract<keyof T, string>>(v as Extract<keyof T, string>),
  )
  return union([first, second, ...rest]) // The type system wants at least two union parameters
}
