import {
  CreateUserKeyPairRequest,
  CreateUserKeyPairResponse,
  GenerateSwarmKeyRequest,
  GenerateSwarmKeyResponse,
  GetNodesDetailsRequest,
  GetNodesDetailsResponse,
  Node,
  QueryRequest,
  QueryResponse,
  SetSettingsRequest,
  ShutdownNodeRequest,
  SignAppManifestRequest,
  SignAppManifestResponse,
} from '../common/types'
import { isLeft } from 'fp-ts/lib/Either'
import reporter from 'io-ts-reporters'
import * as io from 'io-ts'
import * as native from './native'

const runAndDecode = <T>(
  task: native.AsyncTask,
  payload: object,
  decoder: io.Decoder<unknown, T>,
): Promise<T> =>
  new Promise((resolve, reject) => {
    task(JSON.stringify(payload), (err, resp) => {
      if (err) {
        reject(err)
      }
      let obj: object = {}
      try {
        obj = JSON.parse(resp)
      } catch (error) {
        reject(`error parsing JSON response ${error}`)
      }

      const decoded = decoder.decode(obj)
      if (isLeft(decoded)) {
        console.log(`this is the object that couldn't be decoded:`)
        console.log(obj)
        reject(`error decoding object: ${reporter.report(decoded)}`)
        return
      }
      resolve(decoded.right)
    })
  })

const runWithoutResult = (task: native.AsyncTask, payload: object): Promise<void> =>
  new Promise((resolve, reject) => {
    task(JSON.stringify(payload), (err, _) => {
      if (err) {
        reject(err)
      }
      resolve()
    })
  })

const getNodeDetails = (addr: string, timeout: number | null): Promise<Node> =>
  runAndDecode(native.getNodeDetails, { addr, timeout }, Node)

export const getNodesDetails = async (
  reqs: GetNodesDetailsRequest,
): Promise<GetNodesDetailsResponse> =>
  Promise.all(reqs.addrs.map((addr) => getNodeDetails(addr, reqs.timeout)))

export const setSettings = (req: SetSettingsRequest): Promise<void> =>
  runWithoutResult(native.setSettings, req)

export const createUserKeyPair = (
  req: CreateUserKeyPairRequest,
): Promise<CreateUserKeyPairResponse> =>
  runAndDecode(native.createUserKeyPair, req, CreateUserKeyPairResponse)

export const generateSwarmKey = (req: GenerateSwarmKeyRequest): Promise<GenerateSwarmKeyResponse> =>
  runAndDecode(native.generateSwarmKey, req, GenerateSwarmKeyResponse)

export const signAppManifest = (req: SignAppManifestRequest): Promise<SignAppManifestResponse> =>
  runAndDecode(native.signAppManifest, req, SignAppManifestResponse)

export const query = (req: QueryRequest): Promise<QueryResponse> =>
  runAndDecode(native.query, req, QueryResponse)

export const shutdownNode = (req: ShutdownNodeRequest): Promise<void> =>
  runWithoutResult(native.shutdown, req)
