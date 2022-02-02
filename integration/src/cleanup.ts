/**
 * Cleans up all leftover ec2 instances. Triggered regularly by CI
 */
import { EC2Client as EC2 } from '@aws-sdk/client-ec2'
import { cleanUpInstances, cleanUpKeys } from './infrastructure/aws'

const main = async () => {
  if (process.argv.length !== 3) {
    console.error('Please provide the prune age in seconds')
    process.exit(1)
  }

  try {
    const pruneAge: number = +process.argv[2]
    const cutoff = Date.now() - pruneAge * 1000

    console.error(`Deleting instances and KeyPairs started after ${new Date(cutoff).toISOString()}`)

    const ec2 = new EC2({ region: 'eu-central-1' })
    await cleanUpInstances(ec2, cutoff)
    await cleanUpKeys(ec2, cutoff * 1000)
  } catch (ex) {
    console.error('Error', ex)
    process.exit(1)
  }
}

// This will exit immediately, but the API call has already been made so it will correctly terminate the instances
main().then(() => console.log('cleanup finished'))
