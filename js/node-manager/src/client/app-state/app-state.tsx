/* eslint-disable react-hooks/exhaustive-deps */
import React, { useContext, useEffect, useReducer, useState } from 'react'
import {
  signAppManifest,
  createUserKeyPair,
  generateSwarmKey,
  getNodeDetails,
  setSettings,
  waitForNoUserKeysFound,
  shutdownNode,
  query,
  connect,
} from '../util'
import {
  CreateUserKeyPairResponse,
  NodeType,
  Node,
  GenerateSwarmKeyResponse,
  SignAppManifestResponse,
  QueryResponse,
  UiNode,
} from '../../common/types'
import { AppState, AppAction, AppStateKey, AppActionKey } from './types'
import { useAnalytics } from '../analytics'
import { AnalyticsActions } from '../analytics/types'
import { FatalError } from '../../common/ipc'
import { safeErrorToStr } from '../../common/util'
import deepEqual from 'fast-deep-equal'
import { OffsetInfo } from '../offsets'
import { none, Option, some } from 'fp-ts/lib/Option'
import { useStore } from '../store'
import { StoreStateKey } from '../store/types'
import { DEFAULT_TIMEOUT_SEC } from 'common/consts'
import { ipcRenderer } from 'electron'

const POLLING_INTERVAL_MS = 1_000

export const reducer =
  (analytics: AnalyticsActions | undefined) =>
  (state: AppState, action: AppAction): AppState => {
    switch (action.key) {
      case AppActionKey.ShowOverview: {
        if (analytics) {
          analytics.viewedScreen('Overview')
        }
        return { ...state, key: AppStateKey.Overview }
      }
      case AppActionKey.ShowSetupUserKey: {
        if (analytics) {
          analytics.viewedScreen('SetupUserKey')
        }
        return { ...state, key: AppStateKey.SetupUserKey }
      }
      case AppActionKey.ShowAbout: {
        if (analytics) {
          analytics.viewedScreen('About')
        }
        return { ...state, key: AppStateKey.About }
      }
      case AppActionKey.ShowAppSigning: {
        if (analytics) {
          analytics.viewedScreen('AppSigning')
        }
        return { ...state, key: AppStateKey.AppSigning }
      }
      case AppActionKey.ShowNodeAuth: {
        if (analytics) {
          analytics.viewedScreen('NodeAuth')
        }
        return { ...state, key: AppStateKey.NodeAuth }
      }
      case AppActionKey.ShowDiagnostics: {
        if (analytics) {
          analytics.viewedScreen('Diagnostics')
        }
        return { ...state, key: AppStateKey.Diagnostics }
      }
      case AppActionKey.ShowNodeDetail: {
        if (analytics) {
          analytics.viewedScreen('NodeDetail')
        }
        return { ...state, ...action, key: AppStateKey.NodeDetail }
      }
      case AppActionKey.ShowGenerateSwarmKey: {
        if (analytics) {
          analytics.viewedScreen('GenerateSwarmKey')
        }
        return { ...state, ...action, key: AppStateKey.SwarmKey }
      }
      case AppActionKey.ShowPreferences: {
        if (analytics) {
          analytics.viewedScreen('Preferences')
        }
        return { ...state, ...action, key: AppStateKey.Preferences }
      }
      case AppActionKey.ShowQuery: {
        if (analytics) {
          analytics.viewedScreen('Query')
        }
        return { ...state, ...action, key: AppStateKey.Query }
      }
    }
  }

interface Data {
  nodes: UiNode[]
  offsets: Option<OffsetInfo>
  p2n: Map<string, string>
  n2p: Map<string, string>
}

const getPeer = (n: Node, d: Data) => ('peer' in n ? n.peer : d.n2p.get(n.addr))

interface Actions {
  addNodes: (addrs: string[]) => void
  remNodes: (addrs: string[]) => void
  setSettings: (addr: string, settings: object) => Promise<void>
  shutdownNode: (addr: string) => Promise<void>
  createUserKeyPair: (privateKeyPath: string | null) => Promise<CreateUserKeyPairResponse>
  generateSwarmKey: () => Promise<GenerateSwarmKeyResponse>
  signAppManifest: ({
    pathToManifest,
    pathToCertificate,
  }: {
    pathToManifest: string
    pathToCertificate: string
  }) => Promise<SignAppManifestResponse>
  query: (args: { addr: string; query: string }) => Promise<QueryResponse>
}

export type AppDispatch = (action: AppAction) => void
const AppStateContext = React.createContext<
  | {
      state: AppState
      data: Data
      actions: Actions
      dispatch: AppDispatch
    }
  | undefined
>(undefined)

export const AppStateProvider: React.FC<{
  setFatalError: (error: FatalError) => void
}> = ({ children, setFatalError }) => {
  const analytics = useAnalytics()
  const store = useStore()
  const [state, dispatch] = useReducer(reducer(analytics), {
    key: AppStateKey.Overview,
  })
  const [data, setData] = useState<Data>({
    nodes: [],
    offsets: none,
    p2n: new Map(),
    n2p: new Map(),
  })

  const actions: Actions = {
    // Wrap addNodes and add the node as loading as soon as the request
    // is sent
    addNodes: (addrs) => {
      setData((current) => {
        if (analytics) {
          addrs.forEach((addr) => {
            if (current.n2p.get(addr) === undefined) {
              analytics.addedNode()
            }
          })
        }
        for (const addr of addrs) {
          console.log('connecting to', addr)
          connect({ addr, timeout: null })
            .then(({ peer }) => {
              console.log('connected to', addr, peer)
              setData((current) => ({
                ...current,
                p2n: current.p2n.set(peer, addr),
                n2p: current.n2p.set(addr, peer),
              }))
            })
            .catch((err) => {
              console.log('connect error', addr, err)
              setData((current) => ({
                ...current,
                nodes: current.nodes.map((n) =>
                  n.type === NodeType.Loading && n.addr === addr
                    ? { type: NodeType.Unreachable, addr }
                    : n,
                ),
              }))
            })
        }
        return {
          ...current,
          nodes: current.nodes.concat(
            addrs.map((addr) => ({
              type: NodeType.Loading,
              addr,
            })),
          ),
        }
      })
    },
    remNodes: (addrs) => {
      if (analytics) {
        addrs.forEach(() => {
          analytics.removedNode()
        })
      }
      setData((current) => ({
        ...current,
        nodes: current.nodes.filter((n) => !addrs.includes(n.addr)),
      }))
    },
    setSettings: (addr, settings) => {
      if (analytics) {
        analytics.setSettings()
      }
      const peer = data.n2p.get(addr)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : setSettings({ peer, settings })
    },
    shutdownNode: (addr) => {
      if (analytics) {
        analytics.shutdownNode()
      }
      const peer = data.n2p.get(addr)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : shutdownNode({ peer })
    },
    createUserKeyPair: (privateKeyPath) => {
      if (analytics) {
        analytics.createdUserKeyPair(privateKeyPath === null)
      }
      return createUserKeyPair({ privateKeyPath })
    },
    generateSwarmKey: () => {
      if (analytics) {
        analytics.generatedSwarmKey()
      }
      return generateSwarmKey({})
    },
    signAppManifest: ({ pathToManifest, pathToCertificate }) => {
      if (analytics) {
        analytics.signedAppManifest()
      }
      return signAppManifest({
        pathToManifest,
        pathToCertificate,
      })
    },
    query: ({ addr, query: q }) => {
      if (analytics) {
        analytics.queriedEvents(q)
      }
      const peer = data.n2p.get(addr)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : query({ peer, query: q })
    },
  }

  useEffect(() => {
    ipcRenderer.on('onDisconnect', (event, peer) => {
      console.log('onDisconnect', event, peer)
      setData((current) => {
        const addr = data.p2n.get(peer)
        if (addr === undefined) return current
        const p2n = data.p2n
        p2n.delete(peer)
        const n2p = data.n2p
        n2p.delete(addr)
        return { ...current, p2n, n2p }
      })
    })
  }, [])

  useEffect(() => {
    ;(async () => {
      await waitForNoUserKeysFound()
      dispatch({ key: AppActionKey.ShowSetupUserKey })
    })()
  }, [])

  useEffect(() => {
    let unmounted = false
    if (store.key !== StoreStateKey.Loaded) {
      return
    }

    let timeout: ReturnType<typeof setTimeout> | null = null
    const getTimeoutSec =
      (store.key === StoreStateKey.Loaded && store.data.preferences.nodeTimeout) ||
      DEFAULT_TIMEOUT_SEC
    const getDetailsAndUpdate = async () => {
      console.log('getting node information')
      try {
        const toGet = data.nodes.reduce((acc: Record<string, string>, n) => {
          const peer = getPeer(n, data)
          if (peer) acc[peer] = n.addr
          return acc
        }, {})
        const nodes = (
          await Promise.all(
            Object.keys(toGet).map((peer) => getNodeDetails({ peer, timeout: getTimeoutSec })),
          )
        ).map((n) => ({ ...n, addr: toGet[getPeer(n, data) || ''] }))
        const offsetsInfo = OffsetInfo.of(nodes)
        if (!unmounted) {
          if (!deepEqual(data.nodes, nodes) || !deepEqual(data.offsets, some(offsetsInfo))) {
            console.log(`+++ updating app-state/nodes +++`)
            setData({
              ...data,
              offsets: some(offsetsInfo),
              nodes: data.nodes
                .filter((n) => getPeer(n, data) === undefined)
                .concat(
                  nodes.filter((n) =>
                    data.nodes.map((n) => getPeer(n, data)).includes(getPeer(n, data)),
                  ),
                )
                .sort((n1, n2) => n1.addr.localeCompare(n2.addr)),
            })
          }
          timeout = setTimeout(() => {
            getDetailsAndUpdate()
          }, POLLING_INTERVAL_MS)
        }
      } catch (error) {
        const fatalError: FatalError =
          typeof error === 'object' && Object.prototype.hasOwnProperty.call(error, 'shortMessage')
            ? (error as FatalError)
            : { shortMessage: safeErrorToStr(error) }
        setFatalError(fatalError)
      }
    }

    if (state.key !== 'SetupUserKey') {
      timeout = setTimeout(getDetailsAndUpdate, POLLING_INTERVAL_MS)
    }

    return () => {
      unmounted = true
      if (timeout !== null) {
        clearTimeout(timeout)
      }
    }

    // The following line generates a warning; this is known; please don't fix without
    // ensuring that there are no unnecessary re-renders.
  }, [data, state.key, setFatalError, store.key])

  return (
    <AppStateContext.Provider value={{ state, data, actions, dispatch }}>
      {children}
    </AppStateContext.Provider>
  )
}

export const useAppState = () => {
  const c = useContext(AppStateContext)
  if (c === undefined) {
    throw 'AppStateContext is undefined'
  }
  return c
}
