import execa from 'execa'
import { mkProcessLogger } from './mkProcessLogger'
import { windowsActyxOsInstaller } from './settings'
import { Ssh } from './ssh'
import { connectSsh, execSsh, forwardPortsAndBuildClients } from './linux'
import { ActyxOSNode, printTarget, SshAble, Target } from './types'

export const mkWindowsSsh = async (
  nodeName: string,
  target: Target,
  sshParams: SshAble,
  logger: (s: string) => void = console.log,
): Promise<ActyxOSNode> => {
  console.log('setting up Actyx process: %s on %o', nodeName, printTarget(target))

  const ssh = new Ssh(sshParams.host, sshParams.username, sshParams.privateKey)
  // Takes about 300 secs for ssh to be reachable
  await connectSsh(ssh, nodeName, sshParams, 150)

  const binaryPath = await windowsActyxOsInstaller(target.arch)
  const installerPath = String.raw`C:\Actyx-Installer.exe`
  console.log(`${nodeName}: Copying ${binaryPath} ${installerPath}`)
  await ssh.scp(binaryPath, installerPath)

  console.log(`${nodeName}: Installing ${installerPath}`)
  await execSsh(ssh)(
    String.raw`Start-Process -Wait -FilePath ${installerPath} -ArgumentList '/S','/background' -Passthru`,
  )

  console.log(`${nodeName}: Starting Actyx`)
  const actyxOsProc = await startActyxOS(nodeName, logger, ssh)
  return await forwardPortsAndBuildClients(ssh, nodeName, target, actyxOsProc[0], {
    host: 'process',
  })
}

function startActyxOS(
  nodeName: string,
  logger: (s: string) => void,
  ssh: Ssh,
  command = String.raw`Start-Process -Wait -NoNewWindow -FilePath C:\Users\Administrator\AppData\Local\Actyx\actyx.exe -ArgumentList '--working-dir','C:\Users\Administrator\AppData\Local\Actyx\actyx-data','--background'`,
): Promise<[execa.ExecaChildProcess<string>]> {
  // awaiting a Promise<Promise<T>> yields T (WTF?!?) so we need to put it into an array
  return new Promise((res, rej) => {
    const { log, flush } = mkProcessLogger(logger, nodeName, ['NODE_STARTED_BY_HOST'])
    const proc = ssh.exec(command)
    proc.stdout?.on('data', (s: Buffer | string) => {
      if (log('stdout', s)) {
        res([proc])
      }
    })
    proc.stderr?.on('data', (s: Buffer | string) => log('stderr', s))
    proc.on('close', () => {
      flush()
      logger(`node ${nodeName} ActyxOS channel closed`)
      rej('closed')
    })
    proc.on('error', (err: Error) => {
      logger(`node ${nodeName} ActyxOS channel error: ${err}`)
      rej(err)
    })
    proc.on('exit', (code: number, signal: string) => {
      logger(`node ${nodeName} ActyxOS exited with code=${code} signal=${signal}`)
      rej('exited')
    })
  })
}
