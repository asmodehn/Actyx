import { Client, DefaultClientOpts } from '@actyx/os-sdk'
import execa from 'execa'
import { ensureDir, remove } from 'fs-extra'
import path from 'path'
import { CLI } from '../cli'
import { getFreePort } from './checkPort'
import { mkProcessLogger } from './mkProcessLogger'
import { actyxOsDockerImage, currentActyxOsBinary, currentAxBinary, settings } from './settings'
import { ActyxOSNode, Target } from './types'

export const mkNodeLocalProcess = async (
  nodeName: string,
  target: Target,
  logger: (s: string) => void,
): Promise<ActyxOSNode> => {
  const workingDir = path.resolve(settings().tempDir, `${nodeName}-actyx-data`)
  await remove(workingDir)
  await ensureDir(workingDir)
  const binary = await currentActyxOsBinary()

  console.log('node %s starting locally: %s in %s', nodeName, binary, workingDir)

  const [port4001, port4454, port4458] = await Promise.all([0, 0, 0].map(() => getFreePort()))

  const proc = execa(
    binary,
    [
      '--working-dir',
      workingDir,
      '--bind-admin',
      port4458.toString(),
      '--bind-api',
      port4454.toString(),
      '--bind-swarm',
      port4001.toString(),
    ],
    { env: { RUST_BACKTRACE: '1' } },
  )
  const shutdown = async () => {
    console.log('node %s killing process', nodeName)
    proc.kill('SIGTERM')
  }
  const { log, flush } = mkProcessLogger(logger, nodeName, ['NODE_STARTED_BY_HOST'])

  await new Promise<void>((res, rej) => {
    proc.stdout?.on('data', (s: Buffer | string) => log('stdout', s) && res())
    proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s))
    proc.on('close', (code: number, signal: string) =>
      rej(`channel closed, code: ${code}, signal: '${signal}'`),
    )
    proc.on('error', rej)
    proc.on('exit', (code: number, signal: string) =>
      rej(`channel closed, code: ${code}, signal: '${signal}'`),
    )
  }).catch((err) => {
    shutdown()
    flush()
    return Promise.reject(err)
  })
  console.log('node %s Actyx started', nodeName)

  const httpApiOrigin = `http://localhost:${port4454}`
  const opts = DefaultClientOpts()
  opts.Endpoints.EventService.BaseUrl = httpApiOrigin
  const axBinaryPath = await currentAxBinary()
  return {
    name: nodeName,
    target,
    host: 'process',
    ax: await CLI.build(`localhost:${port4458}`, axBinaryPath),
    httpApiClient: Client(opts),
    _private: {
      shutdown,
      axBinaryPath,
      axHost: `localhost:${port4458}`,
      httpApiOrigin,
      apiPond: `ws://localhost:${port4454}/api/v2/events?token=ok`,
      apiSwarmPort: port4001,
    },
  }
}

export const mkNodeLocalDocker = async (
  nodeName: string,
  target: Target,
  gitHash: string,
  logger: (s: string) => void,
): Promise<ActyxOSNode> => {
  const image = actyxOsDockerImage(target.arch, gitHash)
  console.log('node %s starting on local Docker: %s', nodeName, image)

  // exposing the ports and then using -P to use random (free) ports, avoiding trouble
  const command =
    'docker run -d --rm -v /data --expose 4001 --expose 4458 --expose 4454 -P ' + image

  const dockerRun = await execa.command(command)
  const container = dockerRun.stdout

  const shutdown = async () => {
    console.log('node %s shutting down container %s', nodeName, container)
    await execa('docker', ['stop', container])
  }

  try {
    const proc = execa('docker', ['logs', '--follow', container])
    const { log, flush } = mkProcessLogger(logger, nodeName, ['NODE_STARTED_BY_HOST'])

    await new Promise<void>((res, rej) => {
      proc.stdout?.on('data', (s: Buffer | string) => log('stdout', s) && res())
      proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s))
      proc.on('close', (code: number, signal: string) =>
        rej(`channel closed, code: ${code}, signal: '${signal}'`),
      )
      proc.on('error', rej)
      proc.on('exit', (code: number, signal: string) =>
        rej(`channel closed, code: ${code}, signal: '${signal}'`),
      )
    }).catch((err) => {
      flush()
      return Promise.reject(err)
    })
    console.log('node %s ActyxOS started in container %s', nodeName, container)

    const dockerInspect = await execa('docker', ['inspect', container])
    const ports: { [p: string]: { HostIp: string; HostPort: string }[] } = JSON.parse(
      dockerInspect.stdout,
    )[0].NetworkSettings.Ports

    const port = (original: number): string => ports[`${original}/tcp`][0].HostPort
    const axHost = `localhost:${port(4458)}`
    const httpApiOrigin = `http://localhost:${port(4454)}`
    const opts = DefaultClientOpts()
    opts.Endpoints.EventService.BaseUrl = httpApiOrigin

    const axBinaryPath = await currentAxBinary()
    return {
      name: nodeName,
      target,
      host: 'docker',
      ax: await CLI.build(axHost, axBinaryPath),
      httpApiClient: Client(opts),
      _private: {
        shutdown,
        axBinaryPath,
        axHost,
        httpApiOrigin,
        apiPond: `ws://localhost:${port(4454)}/api/v2/events?token=ok`,
        apiSwarmPort: 4001,
      },
    }
  } catch (err) {
    shutdown()
    throw err
  }
}
