import { Client } from '@actyx/os-sdk'
import { CLI } from './cli'
import { ActyxNode } from './infrastructure/types'
import { Arch, Host, OS } from '../jest/types'
import { currentAxBinary } from './infrastructure/settings'

export const mkNodeStub = (
  os: OS,
  arch: Arch,
  host: Host,
  name: string,
  addr = 'localhost',
): Promise<ActyxNode> =>
  currentAxBinary()
    .then((x) => CLI.build(addr, x))
    .then((ax) => ({
      name,
      host,
      target: { os, arch, kind: { type: 'test' }, _private: { cleanup: () => Promise.resolve() } },
      ax,
      httpApiClient: Client(),
      _private: {
        shutdown: () => Promise.resolve(),
        axBinaryPath: '',
        axHost: '',
        httpApiOrigin: '',
        apiPond: '',
        apiSwarmPort: 0,
      },
    }))

export const mkAx = (): Promise<CLI> =>
  mkNodeStub('android', 'aarch64', 'android', 'foo').then((x) => x.ax)

export const mkAxWithUnreachableNode = (): Promise<CLI> =>
  mkNodeStub('android', 'aarch64', 'android', 'foo', '10.42.42.21').then((x) => x.ax)
