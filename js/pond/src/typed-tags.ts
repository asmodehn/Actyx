import { TagIntersection, TagUnion } from './pond-v2-types'

export type Tag<E> = Readonly<{
  // raw tag
  tag: string

  // underlying data type guaranteed by user
  _dataType?: E
}>

export const Tag = {
  mk: <E>(rawTag: string): Tag<E> => ({ tag: rawTag } as Tag<E>),
}

const rawTags = (tags: ReadonlyArray<Tag<unknown>>) => tags.map(x => x.tag)

export const namedSubTags = <E>(tag: Tag<E>, ...path: string[]): Tag<E>[] => {
  let curr = tag.tag
  return path.reduce(
    (tags, element) => {
      curr = curr + ':' + element
      tags.push({ tag: curr })
      return tags
    },
    [tag],
  )
}

export class EmissionTags<E> {
  private tags: ReadonlyArray<string> = []

  constructor() { }

  add<E1>(...tags: Tag<E>[]): EmissionTags<Extract<E, E1>> {
    const r = new EmissionTags<unknown>()
    r.tags = this.tags.concat(rawTags(tags))
    return r as EmissionTags<Extract<E, E1>>
  }

  addPath<E1>(tag: Tag<E>, ...path: string[]): EmissionTags<Extract<E, E1>> {
    const tags = namedSubTags(tag, ...path)
    return this.add(...tags)
  }

  raw(): ReadonlyArray<string> {
    return this.tags
  }
}

export interface TypedTagUnion<E> extends TagUnion {

  readonly _dataType?: E
}

export interface TypedTagIntersection<E> extends TagIntersection {
  and<E1>(...tags: Tag<E1>[]): TypedTagIntersection<Extract<E1, E>>

  readonly _dataType?: E
}

export type TypedTagQuery<E> = TypedTagUnion<E> | TypedTagIntersection<E>

const req = <E>(...tags: Tag<E>[]): TypedTagIntersection<E> => {
  return {
    and: <E1>(...moreTags: Tag<E1>[]) => {
      const cast = [...tags, ...moreTags] as Tag<Extract<E1, E>>[]
      return req<Extract<E1, E>>(...cast)
    },


    type: 'intersection',


    tags: rawTags(tags),

    // raw: () => ({
    // }),
  }
}

const matchAnyOf = <E>(...sets: TypedTagIntersection<E>[]): TypedTagUnion<E> => {
  return {
    type: 'union',
    tags: sets,
  }
}

export const TypedTagQuery = {
  require: req,

  matchAnyOf,
}
