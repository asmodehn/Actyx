import { Event, EventDraft } from '@actyx/os-sdk'
import { Pond } from '@actyx/pond'
import * as PondV1 from 'pondV1'
import { MultiplexedWebsocket } from 'pondV1/lib/eventstore/multiplexedWebsocket'
import { allNodeNames, runOnAll, runOnEach } from '../../infrastructure/hosts'

describe('the Infrastructure', () => {
  test('must create global nodes pool', async () => {
    const status = await runOnEach([{}], (node) => node.ax.nodes.ls())
    expect(status).toMatchObject([
      {
        code: 'OK',
        result: [
          {
            connection: 'reachable',
            version: {
              profile: 'release',
              target: expect.any(String),
              version: '2.0.0_dev',
              gitHash: expect.any(String)
            },
          },
        ],
      },
    ])
    expect(status).toHaveLength(1)
  })

  test('must set up global nodes', async () => {
    const settings = await runOnEach([{}], (node) => node.ax.settings.get('com.actyx'))
    expect(settings).toMatchObject([
      {
        code: 'OK',
        result: {
          admin: {
            logLevels: {
              node: 'DEBUG',
            },
          },
          licensing: {
            apps: {},
            node: 'development',
          },
          api: {
            events: {
              readOnly: false,
            },
          },
          swarm: {
            topic: 'Cosmos integration',
          },
        },
      },
    ])
    expect(settings).toHaveLength(1)
  })

  test.skip('must allow event communication', async () => {
    const events = await runOnEach([{}, {}], async (node) => {
      await node.httpApiClient.eventService.publishPromise({
        eventDrafts: [EventDraft.make('the Infrastructure', node.name, 42)],
      })
      const events: Event[] = []
      const sub = await node.httpApiClient.eventService.subscribeStream({
        subscriptions: [{ streamSemantics: 'the Infrastructure' }],
      })
      for await (const event of sub) {
        events.push(event)
        if (events.length === 2) {
          break
        }
      }
      return events
    })

    expect(events.flat().map((ev) => ev.payload)).toEqual([42, 42, 42, 42])

    const ev1 = events[0].map((ev) => ev.stream.streamName)
    ev1.sort()

    const ev2 = events[1].map((ev) => ev.stream.streamName)
    ev2.sort()

    const expected = allNodeNames().slice(0, 2)
    expected.sort()

    expect(ev1).toEqual(expected)
    expect(ev2).toEqual(expected)
  })

  // FIXME: Pond V1 cannot talk to Event Service V2, this needs to test a V1-compat Pond eventually.
  test.skip('must test Pond v1', async () => {
    const result = await runOnAll([{}], async ([node]) => {
      const pond = await PondV1.Pond.of(new MultiplexedWebsocket({ url: node._private.apiPond }))
      return pond.getNodeConnectivity().take(1).toPromise()
    })
    // cannot assert connected or not connected since we don’t know when this case is run
    expect(typeof result.status).toBe('string')
  })

  test('must test Pond v2', async () => {
    const result = await runOnAll([{}], async ([node]) => {
      const pond = await Pond.of({ url: node._private.apiPond }, {})
      return pond.info().nodeId
    })
    expect(typeof result).toBe('string')
  })
})

describe('scripts', () => {
  test('must allow running sh scripts on linux', async () => {
    await runOnEach([{ os: 'linux' }], async (node) => {
      const script = String.raw`if [[ $(expr 1 + 1) -eq 2 ]]
then
  echo "yay"
  exit 0
else
  exit 1
fi`
      const result = await node.target.execute(script)
      expect(result.exitCode).toBe(0)
      expect(result.stdOut).toBe('yay')
      expect(result.stdErr).toBe('')
    })
  })
  test('must allow running powershell scripts on windows', async () => {
    await runOnEach([{ os: 'windows' }], async (node) => {
      const script = String.raw`$val = 0
while ($val -lt 10) {
  $val++
}
$val + 32
exit 0`
      const result = await node.target.execute(script)
      expect(result.exitCode).toBe(0)
      expect(result.stdOut).toBe('42')
      expect(result.stdErr).toBe('')
    })
  })
})
