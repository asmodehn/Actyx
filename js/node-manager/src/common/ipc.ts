import * as io from 'io-ts'
import {
  SignAppManifestRequest,
  SignAppManifestResponse,
  CreateUserKeyPairRequest,
  CreateUserKeyPairResponse,
  GenerateSwarmKeyRequest,
  GenerateSwarmKeyResponse,
  GetNodesDetailsResponse,
  GetNodesDetailsRequest,
  SetSettingsRequest,
  SetSettingsResponse,
  ShutdownNodeRequest,
  ShutdownNodeResponse,
} from './types'

export const enum IpcFromClient {
  SelectFolder = 'select-folder',
  SelectFile = 'select-file',
  Shutdown = 'shutdown',
  ToggleDevTools = 'toggle-dev-tools',
  LoadStore = 'load-store',
  GetNodesDetails = 'get-nodes-details',
  GetIsDev = 'get-is-dev',
}

export const enum IpcToClient {
  FolderSelected = 'folder-selected',
  FolderSelectedCancelled = 'folder-selected-cancelled',
  FileSelected = 'file-selected',
  FileSelectedCancelled = 'file-selected-cancelled',
  FatalError = 'fatal-error',
  NoUserKeysFound = 'no-user-keys-found',
  StoreLoaded = 'store-loaded',
  GotIsDev = 'got-is-dev',
}

export interface FatalError {
  shortMessage: string
  details?: string
}

export interface RPC<Req, Resp> {
  request: io.Type<Req, object, unknown>
  response: io.Type<Resp, object | void, unknown>
  ipcCode: string
}

const mkRPC = <Req, Resp>(
  ipcCode: string,
  //requestEncoder: io.Encoder<Req, object>,
  request: io.Type<Req, object, unknown>,
  response: io.Type<Resp, object | void, unknown>,
): RPC<Req, Resp> => ({
  ipcCode,
  request,
  response,
})

export const RPC_GetNodesDetails = mkRPC(
  'GetNodesDetails',
  GetNodesDetailsRequest,
  GetNodesDetailsResponse,
)

export const RPC_SetSettings = mkRPC('SetSettings', SetSettingsRequest, SetSettingsResponse)
export const RPC_ShutdownNode = mkRPC('ShutdownNode', ShutdownNodeRequest, ShutdownNodeResponse)

export const RPC_CreateUserKeyPair = mkRPC(
  'CreateUserKeyPair',
  CreateUserKeyPairRequest,
  CreateUserKeyPairResponse,
)

export const RPC_GenerateSwarmKey = mkRPC(
  'GenerateSwarmKey',
  GenerateSwarmKeyRequest,
  GenerateSwarmKeyResponse,
)

export const RPC_SignAppManifest = mkRPC(
  'SignAppManifest',
  SignAppManifestRequest,
  SignAppManifestResponse,
)