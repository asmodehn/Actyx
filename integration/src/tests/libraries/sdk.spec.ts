/**
 * @jest-environment ./dist/integration/src/jest/environment
 */
/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { Actyx, AqlResponse, EventsSortOrder, Tag } from '@actyx/sdk'
import { Observable, lastValueFrom } from 'rxjs'
import { first } from 'rxjs/operators'
import { SettingsInput } from '../../cli/exec'
import { trialManifest } from '../../http-client'
import { runOnEvery, runWithNewProcess } from '../../infrastructure/hosts'
import { randomString } from '../../util'

describe('@actyx/sdk', () => {
  test('node unreachable', async () => {
    await runOnEvery(async (_node) => {
      const wrongConn = Actyx.of(null!, {
        actyxPort: 4453,
      })

      await expect(wrongConn).rejects.toMatchObject({
        message:
          'Error: unable to connect to Actyx at http://localhost:4453/api/v2/node/id. Is the service running? -- Error: request to http://localhost:4453/api/v2/node/id failed, reason: connect ECONNREFUSED 127.0.0.1:4453',
      })
    })
  })

  test('connection without manifest (hello JS users)', async () => {
    await runOnEvery(async (node) => {
      const wrongConn = Actyx.of(null!, {
        actyxPort: node._private.apiPort,
      })

      await expect(wrongConn).rejects.toMatchObject({
        message: 'Invalid request. data did not match any variant of untagged enum AppManifest',
      })
    })
  })

  test('connection with missing manifest signature', async () => {
    await runOnEvery(async (node) => {
      const wrongConn = Actyx.of(
        {
          appId: 'bad.example.bad-app',
          displayName: 'My Example App',
          version: '1.0.0',
        },
        {
          actyxPort: node._private.apiPort,
        },
      )

      await expect(wrongConn).rejects.toMatchObject({
        message: 'Invalid request. data did not match any variant of untagged enum AppManifest',
      })
    })
  })

  test('connection with super bad manifest signature', async () => {
    await runOnEvery(async (node) => {
      const wrongConn = Actyx.of(
        {
          appId: 'bad.example.bad-app',
          displayName: 'My Example App',
          version: '1.0.0',
          signature: 'garbage',
        },
        {
          actyxPort: node._private.apiPort,
        },
      )

      await expect(wrongConn).rejects.toMatchObject({
        message: 'Invalid request. data did not match any variant of untagged enum AppManifest',
      })
    })
  })

  test('connection with invalid manifest signature', async () => {
    await runOnEvery(async (node) => {
      const wrongConn = Actyx.of(
        {
          appId: 'bad.example.bad-app',
          displayName: 'My Example App',
          version: '1.0.0',
          signature:
            // This signature has been doctored by building a special `ax` with `dev_cert.validate_app_id` disabled in signed_app_manifest.rs
            'v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYS3FlRGlvTjZYdnY2SWNXT2JKQmVkY2JWQkcvaVlEZTI0MnovREJYek5UTEFpNzlDdTlBUlYvVkdJV3JER2NRSEZteFVoYytRdk5mRmRtYVIwYkVIQWc9PWlkZXZQdWJrZXl4LTBQMHcwZkJCaktodGZVQ05rQ3YzTy9QamtheGlpb0p6V1B0aWtUVUhYSU5rPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4K1dReWRNMW1sR0MzZkRWS1N0ZDJueXhWLzVKeWEzT01tNC9sV3IrdTF2dkV3eWdUdWl0Qm8waWtlb1JvQVk4TytBclplV1lVdEdzNFNHcWNNWlZCZz09/w==',
        },
        {
          actyxPort: node._private.apiPort,
        },
      )

      await expect(wrongConn).rejects.toMatchObject({
        message:
          'Invalid manifest. AppId \'bad.example.bad-app\' is not allowed in app_domains \'[AppDomain("com.actyx.*"), AppDomain("com.example.*")]\'',
      })
    })
  })

  test('event emission error still allows more emissions, and queries, afterwards', async () => {
    await runOnEvery(async (node) => {
      const actyx = await Actyx.of(trialManifest, {
        actyxPort: node._private.apiPort,
      })

      // The only real error we can produce is "event too large"
      const badEvent = []
      for (let i = 0; i < Math.pow(2, 22); i++) {
        badEvent.push(i)
      }

      const persistBadEvent = actyx.publish(Tag('x').apply(badEvent))

      await expect(persistBadEvent).rejects.toBeTruthy()

      await assertNormalOperationsAndDispose(actyx)
    })
  })

  test('query (complete) error still allows more emissions, and queries, afterwards', async () => {
    await runOnEvery(async (node) => {
      const actyx = await Actyx.of(trialManifest, {
        actyxPort: node._private.apiPort,
      })

      // The only real query error we can produce is bad offsets
      const badOffsets = { foo: 5000 }

      const runBadQuery = actyx.queryKnownRange({ query: Tag('x'), upperBound: badOffsets })

      await expect(runBadQuery).rejects.toBeTruthy()

      await assertNormalOperationsAndDispose(actyx)
    })
  })

  test('query (chunked) error still allows more emissions, and queries, afterwards', async () => {
    await runOnEvery(async (node) => {
      const actyx = await Actyx.of(trialManifest, {
        actyxPort: node._private.apiPort,
      })

      // The only real query error we can produce is bad offsets
      const badOffsets = { foo: 5000 }

      const runBadQuery = new Promise((resolve, reject) =>
        actyx.queryKnownRangeChunked(
          { query: Tag('x'), upperBound: badOffsets },
          20,
          (result) => resolve(result),
          () => reject('err ok'),
        ),
      )

      await expect(runBadQuery).rejects.toBeTruthy()

      await assertNormalOperationsAndDispose(actyx)
    })
  })

  const assertNormalOperationsAndDispose = async (actyx: Actyx) => {
    const okTag = Tag('ok' + Math.random())
    const g = actyx.publish(okTag.apply('hello'))
    await expect(g).resolves.toBeTruthy()

    const q = actyx.queryAllKnown({ query: okTag }).then((x) => x.events[0])
    // Just assert that we get something back
    await expect(q).resolves.toMatchObject({ payload: 'hello' })

    actyx.dispose()
  }

  test('AQL syntax error', async () => {
    await runOnEvery(async (node) => {
      const actyx = await Actyx.of(trialManifest, {
        actyxPort: node._private.apiPort,
      })

      const badQuery = actyx.queryAql('garbage')
      await expect(badQuery).rejects.toBeTruthy()

      const badQueryChunked = new Promise((res, rej) =>
        actyx.queryAqlChunked('garbage', 1, res, rej),
      )
      await expect(badQueryChunked).rejects.toBeTruthy()

      actyx.dispose()
    })
  })

  test('AQL predecessor', async () => {
    await runOnEvery(async (node) => {
      const actyx = await Actyx.of(trialManifest, {
        actyxPort: node._private.apiPort,
      })

      const tagString = randomString()
      const tag = Tag<number>(tagString)
      const evts = await actyx.publish(tag.apply(4, 5))
      const laterEvt = evts[1]

      const predecessor = await new Promise((resolve, reject) => {
        const cancel = actyx.queryAqlChunked(
          {
            order: EventsSortOrder.Descending,
            query: `FEATURES(eventKeyRange) FROM '${tagString}' & to(${laterEvt.eventId})`,
          },
          1,
          (chunk: AqlResponse[]) => {
            resolve(chunk[0])
            cancel() // stop retrieving after getting the first result
          },
          reject,
        )
      })

      expect(predecessor).toMatchObject({
        payload: 4,
      })

      actyx.dispose()
    })
  })

  test('should automatically reconnect if automaticReconnect=true, and also call onConnectionLost', async () =>
    await runWithNewProcess(async (node) => {
      let hookCalled = false

      const actyx = await Actyx.of(trialManifest, {
        actyxPort: node._private.apiPort,
        onConnectionLost: () => {
          hookCalled = true
        },
      })

      try {
        const randomId = String(Math.random())

        const tag = Tag<string>(randomId)

        const p = new Observable((o) =>
          actyx.observeLatest(
            { query: tag },
            (x) => o.next(x),
            (err) => o.error(err),
          ),
        )

        await actyx.publish(tag.apply('event 0'))

        // Wait for the value to arrive
        expect(await lastValueFrom(p.pipe(first()))).toEqual('event 0')

        const expectErr = expect(lastValueFrom(p)).rejects.toEqual(
          new Error(
            '{"Symbol(kTarget)":"WebSocket","Symbol(kType)":"close","Symbol(kCode)":1006,"Symbol(kReason)":"","Symbol(kWasClean)":false}',
          ),
        )
        // Topic change causes WS to be closed. We cannot use `powerCycle` because that gives new port numbers...
        await node.ax.settings.set('/swarm/topic', SettingsInput.FromValue('A different topic'))

        process.stdout.write(node.name + ' waiting for connection closed\n')
        await expectErr
        process.stdout.write(node.name + ' connection closed')

        await new Promise((resolve) => setTimeout(resolve, 3_000))
        expect(hookCalled).toBeTruthy()

        // Assert that reconnection succeeded and we can publish again
        await expect(actyx.publish(tag.apply('qqqq'))).resolves.toMatchObject({ tags: [randomId] })
      } finally {
        actyx.dispose()
      }
    }))
})
