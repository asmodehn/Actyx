import { Client } from '@actyx/os-sdk'
import { CLI } from './ax'
import { ActyxOSNode, Arch, Host, OS, Runtime } from './runner/types'
import { promises as fs } from 'fs'

// TODO: use fs-extra instead
export const exists = async (filePath: string): Promise<boolean> => {
  try {
    await fs.access(filePath)
    return true
  } catch {
    return false
  }
}

export const mkNodeTest = (
  os: OS,
  arch: Arch,
  host: Host,
  runtimes: Runtime[],
  name: string,
  addr = 'localhost',
): ActyxOSNode => {
  return {
    name,
    host,
    runtimes,
    target: { os, arch, kind: { type: 'test' }, _private: { shutdown: () => Promise.resolve() } },
    ax: new CLI(addr),
    actyxOS: Client(),
    _private: {
      shutdown: () => Promise.resolve(),
      axBinary: '',
      axHost: '',
      apiConsole: '',
      apiEvent: '',
      apiPond: '',
    },
  }
}

export const fakeNodeHostUnreachable = mkNodeTest(
  'android',
  'aarch64',
  'android',
  ['webview'],
  'foo',
  '123',
)

export const fakeNodeActyxosUnreachable = mkNodeTest(
  'android',
  'aarch64',
  'android',
  ['webview'],
  'foo',
  'localhost:123',
)
