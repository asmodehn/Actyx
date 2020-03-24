/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Observable } from 'rxjs'
import { AsapScheduler } from 'rxjs/scheduler/AsapScheduler'
import { AsyncScheduler } from 'rxjs/scheduler/AsyncScheduler'
import { QueueScheduler } from 'rxjs/scheduler/QueueScheduler'

export type RxScheduler = QueueScheduler | AsyncScheduler | AsapScheduler

export const unsubscribeOn = <T>(scheduler: QueueScheduler) => {
  return (source: Observable<T>) =>
    new Observable<T>(observer => {
      const subscription = source.subscribe(observer)
      return () => scheduler.schedule(() => subscription.unsubscribe())
    })
}