import { Event, Events, OffsetMap } from '../eventstore/types'
import { LocalSnapshot } from '../types'
import { SimpleReducer } from './types'

export const simpleReducer = <S>(
  onEvent: (oldState: S, event: Event) => S,
  initialState: LocalSnapshot<S>,
): SimpleReducer<S> => {
  // Head is always the latest state known to us
  let head: LocalSnapshot<S> = initialState

  // Advance the head by applying the given event array between (i ..= iToInclusive)
  // without modifying the existing head (WILL modify the `state` inside `head`, though!)
  const appendEvents = (events: Events, fromIdx: number, toIdxInclusive: number) => {
    if (fromIdx > toIdxInclusive) {
      return head
    }

    let i = fromIdx

    let { state, eventKey, cycle } = head
    const offsets = { ...head.psnMap }

    while (i <= toIdxInclusive) {
      const ev = events[i]
      state = onEvent(state, ev)
      OffsetMap.update(offsets, ev)
      eventKey = ev

      i += 1
      cycle += 1
    }

    head = {
      state,
      psnMap: offsets,
      cycle,
      eventKey,
      horizon: head.horizon, // TODO: Detect new horizons from events
    }

    return head
  }

  const setState = (snap: LocalSnapshot<S>) => {
    head = snap
  }

  return {
    appendEvents,
    setState,
  }
}
