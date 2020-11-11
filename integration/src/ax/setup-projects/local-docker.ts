import execa from 'execa'
import settings from '../../../settings'

export const runLocalDocker = async (
  platform: NodeJS.Platform,
  containerName: string,
): Promise<void> => {
  await removeDockerVolume(containerName)
  return supportedPlatforms.includes(platform)
    ? execa
        .command(getSpecificCmd(platform)(containerName))
        .then(() => console.log(`Docker container ${containerName} started`))
    : Promise.reject(`Can not run Docker, platform ${platform} is not supported!`)
}

export const stopLocalDocker = async (containerName: string): Promise<void> => {
  await execa
    .command(`docker stop ${containerName}`)
    .then(() => console.log(`Docker container ${containerName} stopped`))
  await removeDockerVolume(containerName)
}

const removeDockerVolume = async (containerName: string): Promise<void> => {
  const hasContainer = await hasDockerVolume(containerName)
  return hasContainer
    ? execa
        .command(`docker volume rm ${containerName}`)
        .then(() => console.log(`Volume for Docker container ${containerName} was removed`))
        .catch(console.error)
    : console.log(`Docker container ${containerName} has not a mounted volume`)
}

const getSpecificCmd = (x: NodeJS.Platform) => (x === 'linux' ? runOnLinux : runOnWinMac)

const mkRun = (opts: string[] = []) => (containerName: string): string =>
  [
    'docker run',
    '--detach',
    `--name ${containerName}`,
    '--rm',
    '-e AX_DEV_MODE=1',
    `-v ${containerName}:/data`,
    '-p 4001:4001',
    '-p 4457:4457',
    '-p 127.0.0.1:4243:4243',
    '-p 127.0.0.1:4454:4454',
    '--privileged',
    ...opts,
    settings.localDocker.pull,
  ].join(' ')

const runOnLinux = mkRun(['--network=host'])
const runOnWinMac = mkRun()

const supportedPlatforms: NodeJS.Platform[] = ['win32', 'darwin', 'linux']

const hasDockerVolume = async (containerName: string): Promise<boolean> =>
  execa
    .command('docker volume ls --format "{{.Name}}"')
    .then((x) => x.stdout.includes(containerName))
    .catch((error) => {
      console.error(error)
      return false
    })
