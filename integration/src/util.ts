import { ExecaChildProcess, ExecaError, ExecaReturnValue } from 'execa'
import { runOnEvery } from './infrastructure/hosts'
import { mkProcessLogger } from './infrastructure/mkProcessLogger'
import { ActyxNode } from './infrastructure/types'

const getHttpApi = (x: ActyxNode) => x._private.httpApiOrigin

export const run = <T>(f: (httpApi: string) => Promise<T>): Promise<T[]> =>
  runOnEvery((node) => f(getHttpApi(node)))

export const randomString = (): string =>
  Math.random()
    .toString(36)
    .replace(/[^a-z]+/g, '')
    .substr(0, 5)

export const releases = {
  '1.1.5':
    'https://axartifacts.blob.core.windows.net/artifacts/f2e7e414f2a38ba56d64071000b7ac5d3e191d96',
}

export const binaryUrlAndNameForVersion = (
  node: ActyxNode,
  version: keyof typeof releases,
): [string, string] => {
  const basename = version.startsWith('1.') ? 'actyxos' : 'actyx'
  const baseUrl = releases[version]
  const { os, arch } = node.target
  switch (os) {
    case 'linux': {
      return [
        `${baseUrl}/linux-binaries/linux-${arch}/${basename}-linux`,
        `${basename}-linux-${version}`,
      ]
    }
    case 'windows': {
      return [
        `${baseUrl}/windows-binaries/windows-${arch}/${basename}.exe`,
        `${basename}-${version}.exe`,
      ]
    }
    default:
      throw new Error(`cannot get binaries for os=${os}`)
  }
}

const randomBinds = ['--bind-admin', '0', '--bind-api', '0', '--bind-swarm', '0']
const randomBindsWin = randomBinds.map((x) => `'${x}'`).join(',')

export const runActyxVersion = async (
  node: ActyxNode,
  version: keyof typeof releases,
  workdir: string,
): Promise<[ExecaChildProcess]> => {
  const [url, baseExe] = binaryUrlAndNameForVersion(node, version)
  const exe = `${workdir}/${baseExe}`
  const v1 = version.startsWith('1.')
  const wd = v1 ? '--working_dir' : '--working-dir'
  const ts = new Date().toISOString()
  process.stdout.write(`${ts} node ${node.name} starting Actyx ${version} in workdir ${workdir}\n`)
  switch (node.target.os) {
    case 'linux': {
      await node.target.execute('mkdir', ['-p', workdir])
      const download = await node.target.execute('curl', ['-o', exe, url])
      if (download.exitCode !== 0) {
        console.log(`error downloading ${url}:`, download.stderr)
        throw new Error(`error downloading ${url}`)
      }
      await node.target.execute('chmod', ['+x', exe])
      return [node.target.execute(`./${exe}`, [wd, workdir].concat(v1 ? [] : randomBinds))]
    }
    case 'windows': {
      const x = (s: string) => node.target.execute(s, [])
      await x(String.raw`New-Item -ItemType Directory -Path ${workdir} -Force`)
      await x(String.raw`(New-Object System.Net.WebClient).DownloadFile('${url}','${exe}')`)
      const cmd = String.raw`Start-Process -Wait -NoNewWindow -FilePath ${exe} -ArgumentList '${wd}','${workdir}'${
        v1 ? '' : ',' + randomBindsWin
      }`
      return [x(cmd)]
    }
    default:
      throw new Error(`cannot run specific Actyx version on os=${node.target.os}`)
  }
}

export const runActyx = (node: ActyxNode, workdir: string): ExecaChildProcess => {
  const ts = new Date().toISOString()
  process.stdout.write(`${ts} node ${node.name} starting current Actyx in workdir ${workdir}\n`)
  switch (node.target.os) {
    case 'linux': {
      return node.target.execute(
        `./${node._private.actyxBinaryPath}`,
        ['--working-dir', workdir].concat(randomBinds),
      )
    }
    case 'windows': {
      const cmd = String.raw`Start-Process -Wait -NoNewWindow -FilePath "${node._private.actyxBinaryPath}" -ArgumentList "--working-dir","${workdir}",${randomBindsWin}`
      return node.target.execute(cmd, [])
    }
    default:
      throw new Error(`cannot start Actyx on os=${node.target.os}`)
  }
}

export const runUntil = (
  proc: ExecaChildProcess,
  nodeName: string,
  triggers: string[],
  timeout: number,
): Promise<ExecaReturnValue | ExecaError | string[]> =>
  new Promise<ExecaReturnValue | ExecaError | string[]>((res) => {
    const logs: string[] = []
    setTimeout(() => res(logs), timeout)
    const { log } = mkProcessLogger((s) => logs.push(s), nodeName, triggers)
    proc.stdout?.on('data', (buf) => {
      if (log('stdout', buf)) {
        res(logs)
      }
    })
    proc.stderr?.on('data', (buf) => {
      if (log('stderr', buf)) {
        res(logs)
      }
    })
    proc.then(res, res)
  }).finally(() => proc.kill())
