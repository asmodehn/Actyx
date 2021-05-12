import { CLI } from '../cli'
import { ApiClient } from '@actyx/os-sdk'
import { Arch, Host, OS } from '../../jest/types'

export type Target = {
  os: OS
  arch: Arch
  kind: TargetKind
  execute: (script: string) => Promise<{ exitCode: number; stdOut: string; stdErr: string }>
  // Run in the virtualization layer in which the actyx process runs, if it
  // doesn't run directly on the host. For Actyx on Docker, this provides direct
  // access to the Docker container in which Actyx is running via `docker exec`.
  // For an Android emulator, this provides adb access.
  executeInContainer?: (
    script: string,
  ) => Promise<{ exitCode: number; stdOut: string; stdErr: string }>
  _private: {
    cleanup: () => Promise<void>
    // Helper to get `executeInContainer` over the process boundary. This is a
    // prefix, with which `executeInContainer` can be reconstructed.
    executeInContainerPrefix?: string
  }
}

export type SshAble = {
  host: string
  username?: string
  privateKey?: string
}

export type LocalTargetKind = Readonly<{
  type: 'local'
  reuseWorkingDirIfExists?: boolean
}>

export type TargetKind =
  | ({ type: 'aws'; instance: string; privateAddress: string } & SshAble)
  | ({ type: 'ssh' } & SshAble)
  | LocalTargetKind
  | { type: 'test' }

export const printTarget = (t: Target): string => {
  const kind = t.kind
  switch (kind.type) {
    case 'aws': {
      return `AWS ${kind.instance} ${kind.host} ${t.os}/${t.arch}`
    }
    case 'ssh': {
      return `borrowed (SSH) ${kind.host} ${t.os}/${t.arch}`
    }
    case 'local': {
      return `borrowed (local) ${t.os}/${t.arch}`
    }
    case 'test': {
      return `test ${t.os}/${t.arch}`
    }
  }
}

export type NodeSelection = {
  os?: OS
  arch?: Arch
  host?: Host
}

export type ActyxNode = Readonly<{
  name: string
  target: Target
  host: Host
  ax: CLI
  httpApiClient: ApiClient
  _private: Readonly<{
    shutdown: () => Promise<void>
    axBinaryPath: string
    axHost: string
    httpApiOrigin: string
    apiPond: string
    apiSwarmPort: number
  }>
}>

export type AwsKey = {
  keyName: string
  privateKey: string
  publicKeyPath: string
}
