import fetch from 'node-fetch'
import {
  getToken,
  mkEventsPath,
  trialManifest,
  API_V2_PATH,
  EVENTS_PATH,
  OFFSETS_SEG,
  AUTH_SEG,
} from '../../http-client'
import WebSocket from 'ws'
import { run, getHttpApi, power_cycle } from '../../util'
import { AppManifest } from '@actyx/pond'
import { SettingsInput } from '../../cli/exec'
import { waitForNodeToBeConfigured } from '../../retry'
import { runWithNewProcess } from '../../infrastructure/hosts'

const UNAUTHORIZED_TOKEN =
  'AAAAWaZnY3JlYXRlZBsABb3ls11m8mZhcHBfaWRyY29tLmV4YW1wbGUubXktYXBwZmN5Y2xlcwBndmVyc2lvbmUxLjAuMGh2YWxpZGl0eRkBLGlldmFsX21vZGX1AQv+4BIlF/5qZFHJ7xJflyew/CnF38qdV1BZr/ge8i0mPCFqXjnrZwqACX5unUO2mJPsXruWYKIgXyUQHwKwQpzXceNzo6jcLZxvAKYA05EFDnFvPIRfoso+gBJinSWpDQ=='

const getOffsets = (httpApi: string, authHeaderValue?: string) =>
  fetch(httpApi + API_V2_PATH + EVENTS_PATH + OFFSETS_SEG, {
    method: 'get',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/json',
      ...(authHeaderValue ? { Authorization: authHeaderValue } : {}),
    },
  })

describe('auth http', () => {
  const signedManifest: AppManifest = {
    appId: 'com.actyx.auth-test',
    displayName: 'auth test app',
    version: 'v0.0.1',
    signature:
      'v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w==',
  }

  it('auth flow signed manifest with node in prod mode', () =>
    runWithNewProcess(async (node) => {
      const httpApi = getHttpApi(node)

      const set = async (scope: string, value: unknown): Promise<void> => {
        expect(
          await node.ax.settings.set(`/licensing/${scope}`, SettingsInput.FromValue(value)),
        ).toMatchCodeOk()
        await waitForNodeToBeConfigured(node)
      }

      const setAppLicense = (license: string): Promise<void> =>
        set('apps', { [signedManifest.appId]: license })

      const get = (expected: unknown) =>
        getToken(signedManifest, httpApi)
          .then((x) => x.json())
          .then((x) => {
            expect(x).toEqual(expected)
            return x
          })

      const getErr = (msg: string) =>
        get({
          code: 'ERR_APP_UNAUTHORIZED',
          message: `'com.actyx.auth-test' is not authorized. ${msg}. Provide a valid app license to the node.`,
        })

      const offsets = async (token: string) => {
        const resp = await getOffsets(node._private.httpApiOrigin, 'Bearer ' + token)
        return {
          status: resp.status,
          json: await resp.json(),
        }
      }

      // should get token when node is not in prod mode
      const { token: token1 } = await get({ token: expect.any(String) })
      expect(await offsets(token1)).toEqual({
        status: 200,
        json: { present: expect.any(Object), toReplicate: expect.any(Object) },
      })

      // should fail when node in prod mode without app license
      await set('node', 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx')
      await getErr('License not found for app')

      // FIXME: previous token should actually be invalidated
      expect(await offsets(token1)).toEqual({
        status: 200,
        json: { present: expect.any(Object), toReplicate: expect.any(Object) },
      })

      // let's set malformed licence for our app id
      await setAppLicense(
        'MALFORMED_LICENSE_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
      )
      await getErr('Could not parse license')

      // try out with falsified license
      await setAppLicense(
        'v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZHNjb20uYWN0eXguYXV0aC10ZXN0aWV4cGlyZXNBdHQxOTcxLTAxLTAxVDAwOjAxOjAxWmljcmVhdGVkQXR0MTk3MC0wMS0wMVQwMDowMTowMVppc2lnbmF0dXJleFg1dmEvQ3NYWlk3TUV6VVJ0SUEwVm9mL3R1T3FlejZCN3FYby9JNTl4T0NkUDNwUFVabGZEekZPbExIK09oZXJjWGkwRTJ1RXFnZ2x1cUdyaGFDVVhDZz09aXJlcXVlc3RlcqFlZW1haWx0Y3VzdG9tZXJAZXhhbXBsZS5jb23/',
      )
      await getErr('Could not validate license')

      // use proper app manifest
      await setAppLicense(
        'v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZHNjb20uYWN0eXguYXV0aC10ZXN0aWV4cGlyZXNBdHQxOTcxLTAxLTAxVDAwOjAxOjAxWmljcmVhdGVkQXR0MTk3MC0wMS0wMVQwMDowMTowMVppc2lnbmF0dXJleFhBQWRSd1U4UTZlb3JLY0N3SjE1T0t4OWVPQ0kxNjN3MFhwTFpHWkNPUWlDWUZlYkR1cFlBbWlNOVhsb3dDYWw5dUtuSWhRelkzSUo2RkdUbEtJMStEUT09aXJlcXVlc3RlcqFlZW1haWx0Y3VzdG9tZXJAZXhhbXBsZS5jb23/',
      )

      const { token: token2 } = await get({ token: expect.any(String) })
      expect(await offsets(token2)).toEqual({
        status: 200,
        json: { present: expect.any(Object), toReplicate: expect.any(Object) },
      })

      await set('node', 'development')

      // FIXME: previous token should actually be invalidated
      expect(await offsets(token1)).toEqual({
        status: 200,
        json: { present: expect.any(Object), toReplicate: expect.any(Object) },
      })
    }))

  it('should get token for signed manifest', () =>
    run((httpApi) =>
      getToken(signedManifest, httpApi)
        .then((x) => x.json())
        .then((x) =>
          expect(x).toEqual({
            token: expect.any(String),
          }),
        ),
    ))

  it('should fail to get token for falsified manifest', () =>
    run((httpApi) =>
      getToken({ ...signedManifest, version: '1' }, httpApi)
        .then((resp) => {
          expect(resp.status).toEqual(400)
          return resp.json()
        })
        .then((json) =>
          expect(json).toEqual({
            code: 'ERR_MANIFEST_INVALID',
            message:
              'Invalid manifest. Failed to validate app manifest. Invalid signature for provided input.',
          }),
        ),
    ))

  it('should fail for malformed requests', () =>
    run((httpApi) =>
      fetch(httpApi + API_V2_PATH + AUTH_SEG, {
        method: 'post',
        body: JSON.stringify({ malformed: true }),
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
        },
      })
        .then((resp) => {
          expect(resp.status).toEqual(400)
          return resp.json()
        })
        .then((json) =>
          expect(json).toEqual({
            code: 'ERR_BAD_REQUEST',
            message: 'Invalid request. data did not match any variant of untagged enum AppManifest',
          }),
        ),
    ))

  it('should fail when the manifest is invalid', () =>
    run((httpApi) =>
      fetch(httpApi + API_V2_PATH + AUTH_SEG, {
        method: 'post',
        body: JSON.stringify({
          appId: 'com.actyx.my-app',
          displayName: 'Mine!',
          version: '0.8.5',
          signature:
            'v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w==',
        }),
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
        },
      })
        .then((resp) => {
          expect(resp.status).toEqual(400)
          return resp.json()
        })
        .then((json) =>
          expect(json).toEqual({
            code: 'ERR_MANIFEST_INVALID',
            message:
              'Invalid manifest. Failed to validate app manifest. Invalid signature for provided input.',
          }),
        ),
    ))

  it('should fail when token not authorized', () =>
    run((httpApi) =>
      getOffsets(httpApi, 'Bearer ' + UNAUTHORIZED_TOKEN)
        .then((resp) => {
          expect(resp.status).toEqual(401)
          return resp.json()
        })
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_TOKEN_UNAUTHORIZED',
            message: 'Unauthorized token.',
          }),
        ),
    ))

  it('should fail when auth header has wrong value', () =>
    run((httpApi) =>
      getOffsets(httpApi, 'Foo bar')
        .then((resp) => {
          expect(resp.status).toEqual(401)
          return resp.json()
        })
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_UNSUPPORTED_AUTH_TYPE',
            message: 'Unsupported authentication type \'Foo\'. Only "Bearer" is supported.',
          }),
        ),
    ))

  it('should fail when token is invalid', () =>
    run((httpApi) =>
      getOffsets(httpApi, 'Bearer invalid')
        .then((resp) => {
          expect(resp.status).toEqual(400)
          return resp.json()
        })
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_TOKEN_INVALID',
            message:
              "Invalid token: 'invalid'. Cannot parse token bytes. Please provide a valid bearer token.",
          }),
        ),
    ))

  it('should fail when authorization header is missing', () =>
    run((httpApi) =>
      getOffsets(httpApi)
        .then((resp) => {
          expect(resp.status).toEqual(401)
          return resp.json()
        })
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_MISSING_AUTH_HEADER',
            message: '"Authorization" header is missing.',
          }),
        ),
    ))

  it('should fail for a valid token when node is cycled', () =>
    runWithNewProcess(async (node) => {
      if (node.target.os === 'windows') {
        // FIXME #6990 releasing the LockFile is hard to ensure
        return
      }
      const token = await getToken(trialManifest, node._private.httpApiOrigin)
        .then((x) => x.json())
        .then((x) => x.token)
      const offsets = (origin: string) => getOffsets(origin, 'Bearer ' + token)

      // assert we can access event service
      const response = await offsets(node._private.httpApiOrigin).then((resp) => resp.json())
      expect(response).toEqual({ present: expect.any(Object), toReplicate: expect.any(Object) })

      // power cycle the node
      await power_cycle(node)

      const result = await offsets(node._private.httpApiOrigin).then((resp) => {
        expect(resp.status).toEqual(401)
        return resp.json()
      })
      expect(result).toEqual({ code: 'ERR_TOKEN_EXPIRED', message: 'Expired token.' })
    }))

  // TODO: test expired token response, idea is to add a parameter to the auth call that can shorten the token lifetime
})

describe('auth ws', () => {
  const mkWs = (path: string, f: (ws: WebSocket, resolve: () => void) => void): Promise<void[]> =>
    run((httpApi) => {
      const ws = new WebSocket(httpApi + mkEventsPath(path))
      return new Promise<void>((resolve) => {
        f(ws, resolve)
      })
    })

  const expectFailure = (path: string, status: number): Promise<void[]> =>
    mkWs(path, (ws, resolve) => {
      ws.on('error', (x) => {
        expect(x.message).toEqual(`Unexpected server response: ${status}`)
        resolve()
      })
    })

  it('should fail when token is missing', () => expectFailure('', 401))

  it('should fail when token is not authorized', () => expectFailure(`?${UNAUTHORIZED_TOKEN}`, 401))

  it('should fail when using wrong path', () =>
    expectFailure(`/wrong_path?token-does-not-matter`, 404))

  it('should get token for a trial manifest and successfully use it', () =>
    run((httpApi) =>
      getToken(trialManifest, httpApi)
        .then((authResponse) => authResponse.json())
        .then((x) => {
          const ws = new WebSocket(httpApi + mkEventsPath(`?${x.token}`))
          const message = {
            type: 'request',
            serviceId: 'offsets',
            requestId: 1,
            payload: null,
          }
          const responses: unknown[] = []
          return new Promise<void>((resolve) => {
            ws.on('message', (x) => {
              responses.push(JSON.parse(x.toString()))
              if (responses.length === 2) {
                expect(responses).toMatchObject([{ type: 'next' }, { type: 'complete' }])
                ws.terminate()
                resolve()
              }
            })
            ws.onopen = () => ws.send(JSON.stringify(message))
          })
        }),
    ))
})
