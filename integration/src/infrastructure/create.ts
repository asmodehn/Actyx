/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { ensureDirSync } from 'fs-extra'
import { EC2 } from 'aws-sdk'
import { createInstance, instanceToTarget } from './aws'
import { mkNodeSshDocker, mkNodeSshProcess } from './linux'
import { ActyxNode, AwsKey, SshAble, Target, TargetKind } from './types'
import { CreateEC2, currentArch, currentOS, HostConfig } from '../../jest/types'
import { mkNodeLocalDocker, mkNodeLocalProcess } from './local'
import { LogEntry, MyGlobal } from '../../jest/setup'
import fs, { readFileSync } from 'fs'
import path from 'path'
import { makeWindowsInstallScript, mkWindowsSsh } from './windows'
import { mkExecute } from '.'
import { mkNodeSshAndroid } from './android'

const createAwsInstance = async (
  ec2: EC2,
  prepare: CreateEC2,
  key: AwsKey,
  hostname: string,
  runIdentifier: string,
  volumeSizeGib?: number,
  userData?: string,
): Promise<Target> => {
  const instance = await createInstance(ec2, {
    InstanceType: prepare.instance,
    ImageId: prepare.ami,
    KeyName: key.keyName,
    TagSpecifications: [
      {
        ResourceType: 'instance',
        Tags: [
          { Key: 'Name', Value: hostname },
          { Key: 'ci_run', Value: runIdentifier },
        ],
      },
    ],
    UserData: userData,
    BlockDeviceMappings: [
      {
        DeviceName: '/dev/sda1',
        Ebs: { VolumeSize: volumeSizeGib, DeleteOnTermination: true },
      },
    ],
  })
  return instanceToTarget(instance, prepare, key, ec2)
}

const installProcess = async (target: Target, host: HostConfig, logger: (line: string) => void) => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws':
    case 'ssh':
      if (host.install === 'windows') {
        return await mkWindowsSsh(host.name, target, kind, logger)
      } else {
        return await mkNodeSshProcess(host.name, target, kind, logger)
      }

    case 'local':
      return await mkNodeLocalProcess(host.name, target, logger)

    default:
      console.error('unknown kind:', kind)
  }
}

const installDocker = async (
  target: Target,
  host: HostConfig,
  logger: (line: string) => void,
  gitHash: string,
) => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws':
    case 'ssh':
      return await mkNodeSshDocker(host.name, target, kind, logger, gitHash)

    case 'local':
      return await mkNodeLocalDocker(host.name, target, gitHash, logger)

    default:
      console.error('unknown kind:', kind)
  }
}

const installAndroidEmulator = async (
  target: Target,
  host: HostConfig,
  logger: (line: string) => void,
) => {
  const kind = target.kind
  switch (kind.type) {
    case 'aws':
    case 'ssh':
      return await mkNodeSshAndroid(host.name, target, kind, logger)

    default:
      console.error('unsupported kind: ', kind)
  }
}
/**
 * Create a new node from the HostConfig that describes it. This can entail spinning up an EC2
 * host or it can mean using locally available resources like a Docker daemon.
 *
 * @param hostConfig
 */
export const createNode = async (hostConfig: HostConfig): Promise<ActyxNode | undefined> => {
  const {
    ec2,
    key,
    gitHash,
    thisTestEnvNodes,
    settings: { logToStdout },
    runIdentifier,
  } = (<MyGlobal>global).axNodeSetup

  let target: Target | undefined = undefined

  const { prepare, name: hostname } = hostConfig
  switch (prepare.type) {
    case 'create-aws-ec2': {
      if (typeof key === 'undefined' || typeof ec2 === 'undefined') {
        throw 'No AWS EC2 Keypair was created. Are you authenticated with AWS?'
      }
      switch (hostConfig.install) {
        case 'windows': {
          const pubKey = readFileSync(key.publicKeyPath)
          const enableSshScript = makeWindowsInstallScript(pubKey.toString('utf8'))
          const userData = Buffer.from(enableSshScript).toString('base64')
          target = await createAwsInstance(
            ec2,
            prepare,
            key,
            hostname,
            runIdentifier,
            undefined,
            userData,
          )
          break
        }
        case 'android': {
          target = await createAwsInstance(ec2, prepare, key, hostname, runIdentifier, 32)
          break
        }
        case 'linux':
        case 'docker': {
          target = await createAwsInstance(ec2, prepare, key, hostname, runIdentifier)
          break
        }
      }
      break
    }
    case 'ssh': {
      const { os, arch, user, privateKeyPath, host } = prepare
      const sshable: SshAble = {
        host,
        privateKey: privateKeyPath,
        username: user,
      }
      const kind: TargetKind = { type: 'ssh', ...sshable }
      const execute = mkExecute(os, kind)
      target = {
        arch,
        os,
        execute,
        kind,
        _private: { cleanup: () => Promise.resolve() },
      }
      break
    }
    case 'local': {
      console.log('node %s using the local system', hostConfig.name)
      const shutdown = () => Promise.resolve()
      const os = currentOS()
      const kind = { type: 'local' as const }
      const execute = mkExecute(os, kind)

      target = {
        os,
        arch: currentArch(),
        execute,
        _private: {
          cleanup: shutdown,
        },
        kind,
      }
      break
    }
  }

  const logs: LogEntry[] = []
  const logger = (line: string) => {
    logs.push({ time: new Date(), line })
  }

  try {
    let node: ActyxNode | undefined
    switch (hostConfig.install) {
      case 'linux':
      case 'windows':
        if (target === undefined) {
          console.error('no recipe to prepare node %s', hostConfig.name)
          return
        }
        node = await installProcess(target, hostConfig, logger)
        break
      case 'docker':
        if (target === undefined) {
          console.error('no recipe to prepare node %s', hostConfig.name)
          return
        }
        node = await installDocker(target, hostConfig, logger, gitHash)
        break

      case 'android':
        if (target === undefined) {
          console.error('no recipe to prepare node %s', hostConfig.name)
          return
        }
        node = await installAndroidEmulator(target, hostConfig, logger)
        break

      default:
        return
    }

    if (node === undefined) {
      console.error('no recipe to install node %s', hostConfig.name)
    } else {
      const orig_shutdown = node._private.shutdown
      const shutdown = async () => {
        await orig_shutdown().catch((error) =>
          console.error('node %s error while shutting down:', hostConfig.name, error),
        )
        const logFilePath = mkLogFilePath(runIdentifier, hostConfig)
        const [logSink, flush] = logToStdout
          ? [process.stdout.write, () => ({})]
          : appendToFile(logFilePath)

        process.stdout.write(
          `\n****\nlogs for node ${hostConfig.name}${
            logToStdout ? '' : ` redirected to "${logFilePath}"`
          }\n****\n\n`,
        )
        console.log('node %s: Flushing logs', hostConfig.name)
        for (const entry of logs) {
          logSink(`${entry.time.toISOString()} ${entry.line}\n`)
        }
        flush()
        console.log('node %s: Flushed logs', hostConfig.name)
        logs.length = 0
      }

      node = { ...node, _private: { ...node._private, shutdown } }
    }

    if (thisTestEnvNodes !== undefined && node !== undefined) {
      thisTestEnvNodes.push(node)
    }

    return node
  } catch (e) {
    console.error('node %s error while setting up:', hostConfig.name, e)
    for (const entry of logs) {
      process.stdout.write(`${entry.time.toISOString()} ${entry.line}\n`)
    }
    await target?._private.cleanup()
    throw e
  }
}

// Constructs a log file path for a given `runId` and a `host`. Will create any
// needed folders.
const mkLogFilePath = (runId: string, host: HostConfig) => {
  const folder = path.resolve('logs', runId)
  ensureDirSync(folder)
  return path.resolve(folder, host.name)
}

const appendToFile = (fileName: string): [(_: string) => void, () => void] => {
  const fd = fs.openSync(fileName, 'a')
  return [(line: string) => fs.writeSync(fd, line), () => fs.closeSync(fd)]
}
