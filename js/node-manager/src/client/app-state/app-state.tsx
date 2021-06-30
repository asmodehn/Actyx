import React, { useContext, useEffect, useReducer, useState } from 'react'
import {
  signAppManifest,
  createUserKeyPair,
  generateSwarmKey,
  getNodesDetails,
  setSettings,
  waitForNoUserKeysFound,
  shutdownNode,
} from '../util'
import {
  CreateUserKeyPairResponse,
  NodeType,
  Node,
  GenerateSwarmKeyResponse,
  SignAppManifestResponse,
} from '../../common/types'
import { AppState, AppAction, AppStateKey, AppActionKey } from './types'
import { useAnalytics } from '../analytics'
import { AnalyticsActions } from '../analytics/types'
import { FatalError } from '../../common/ipc'
import { safeErrorToStr } from '../../common/util'

const POLLING_INTERVAL_MS = 1000

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
    }
  }

interface Data {
  nodes: Node[]
}

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
}

export type AppDispatch = (action: AppAction) => void
const AppStateContext =
  React.createContext<
    | {
        state: AppState
        data: Data
        actions: Actions
        dispatch: AppDispatch
      }
    | undefined
  >(undefined)

export const AppStateProvider: React.FC<{ setFatalError: (error: FatalError) => void }> = ({
  children,
  setFatalError,
}) => {
  const analytics = useAnalytics()
  const [state, dispatch] = useReducer(reducer(analytics), {
    key: AppStateKey.Overview,
  })
  const [data, setData] = useState<Data>({ nodes: [] })

  const actions: Actions = {
    // Wrap addNodes and add the node as loading as soon as the request
    // is sent
    addNodes: (addrs) => {
      setData((current) => {
        if (analytics) {
          addrs.forEach((addr) => {
            if (current.nodes.find((node) => node.addr !== addr)) {
              analytics.addedNode()
            }
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
      return setSettings({ addr, settings })
    },
    shutdownNode: (addr) => {
      if (analytics) {
        analytics.shutdownNode()
      }
      return shutdownNode({ addr })
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
  }

  useEffect(() => {
    ;(async () => {
      await waitForNoUserKeysFound()
      dispatch({ key: AppActionKey.ShowSetupUserKey })
    })()
  }, [])

  useEffect(() => {
    let unmounted = false

    let timeout: ReturnType<typeof setTimeout> | null = null
    const getDetailsAndUpdate = async () => {
      try {
        const nodes = await getNodesDetails({ addrs: data.nodes.map((n) => n.addr) })
        if (!unmounted) {
          setData((current) => ({
            ...current,
            nodes: current.nodes
              .filter((n) => !nodes.map((n) => n.addr).includes(n.addr))
              .concat(nodes),
          }))
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
  }, [data.nodes, state.key, setFatalError])

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
