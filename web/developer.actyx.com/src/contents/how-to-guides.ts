/* 
This file contains all categories and their contents of how-to guides.
*/

const createHowTos = () => {
  const howToGuides = [
    {
      category: 'Local Development Setup',
      description:
        'Get your local environment set up; install Actyx and the SDK, start a new project, setup your environment, and debug common errors.',
      contents: [
        {
          title: 'Installing and starting Actyx',
          link: '/docs/how-to/local-development/install-actyx',
        },
        {
          title: 'Starting a new project',
          link: '/docs/how-to/local-development/starting-a-new-project',
        },
        {
          title: 'Setting up your JS environment',
          link: '/docs/how-to/local-development/setting-up-your-environment',
        },
        {
          title: 'Installing Actyx CLI and Node Manager',
          link: '/docs/how-to/local-development/install-cli-node-manager',
        },
        /* {
          title: 'Obtaining your development certificate',
          link: '/docs/how-to/local-development/obtaining-a-development-certificate',
        }, */
      ],
    },
    /* {
      category: 'Process Logic',
      description:
        'Implement processes by writing local twins. Execute these processes and integrate them into the real world through apps.',
      contents: [
        {
          title: 'Publishing to event streams',
          link: '',
        },
        {
          title: 'Subscribing to and querying event streams',
          link: '',
        },
        {
          title: 'Computing a state from events',
          link: '',
        },
        {
          title: 'Automating decision making',
          link: '',
        },
        {
          title: 'Dealing with network partitions',
          link: '',
        },
        {
          title: 'Modelling processes in twins',
          link: '',
        },
        {
          title: 'Transferring twins into code',
          link: '',
        },
      ],
    }, */
    {
      category: 'Using Actyx Pond to its full potential',
      description:
        'Actyx Pond is your programming framework for implementing distributed processes and automating factory solutions.',
      contents: [
        {
          title: 'Introduction',
          link: '/docs/how-to/actyx-pond/introduction',
        },
        {
          title: 'Getting Started with the Pond',
          link: '/docs/how-to/actyx-pond/getting-started',
        },
        {
          title: 'Learning to work with Actyx Pond in 10 Steps',
          link: '/docs/how-to/actyx-pond/guides/hello-world',
        },
        {
          title: 'Fish Parameters',
          link: '/docs/how-to/actyx-pond/fish-parameters/on-event',
        },
        {
          title: 'Pond in Depth Guides for advanced users',
          link: '/docs/how-to/actyx-pond/in-depth/tag-type-checking',
        },
      ],
    },
    /* {
      category: 'Getting data into and out of Actyx',
      description:
        'Implement processes by writing local twins. Execute these processes and integrate them into the real-world through apps.',
      contents: [
        {
          title: 'Integrating logic with a user interface',
          link: '',
        },
        {
          title: 'Integrating with other software',
          link: '',
        },
        {
          title: 'Integrating with front-end frameworks (react, angular)',
          link: '',
        },
        {
          title: 'Integrating with machines',
          link: '',
        },
        {
          title: 'Integrating with ERPs',
          link: '',
        },
        {
          title: 'Integrating with business intelligence / analytics',
          link: '',
        },
      ],
    },
    {
      category: 'Testing with Actyx',
      description:
        'Add unit and end-to-end tests for your twins and apps. Use well established tools such as Jest or Cypress. Set up a CI/CD pipeline.',
      contents: [
        {
          title: 'Design a testing pipeline',
          link: '',
        },
        {
          title: 'Unit-test with Jest',
          link: '',
        },
        {
          title: 'Unit-test with Cypress',
          link: '',
        },
        {
          title: 'Set up integration testing',
          link: '',
        },
        {
          title: 'Set up a CI/CD pipeline',
          link: '',
        },
      ],
    }, */
    {
      category: 'Swarms',
      description: 'Setup swarms and connect nodes.',
      contents: [
        {
          title: 'Setup a swarm and configure nodes to join it',
          link: 'swarms/setup-swarm',
        },
        {
          title: 'Setup a bootstrap node to connect nodes to each other',
          link: 'swarms/setup-bootstrap-node',
        },
      ],
    },
    {
      category: 'Packaging Apps for different platforms',
      description: 'Discover the possibilities you have when packaging apps for Actyx.',
      contents: [
        {
          title: 'Mobile apps',
          link: '/docs/how-to/packaging/mobile-apps',
        },
        {
          title: 'Desktop apps',
          link: '/docs/how-to/packaging/desktop-apps',
        },
        {
          title: 'Headless apps',
          link: '/docs/how-to/packaging/headless-apps',
        },
        /* {
          title: 'Deploying to production',
          link: '',
        },
        {
          title: 'Updating an Actyx solution',
          link: '',
        }, */
      ],
    },
    {
      category: 'Monitoring & Debugging',
      description:
        'Debug your code locally on your machine or remote-monitor installations with 3rd party mobile device management tools.',
      contents: [
        {
          title: 'Accessing node logs',
          link: '',
        },
        {
          title: 'Publishing and accessing app logs',
          link: '',
        },
        {
          title: "Using the node's connectivity status for debugging",
          link: '',
        },
        {
          title: 'Using 3rd party mobile device management tools',
          link: '',
        },
        {
          title: 'Accelerating your workflow with bash',
          link: '',
        },
      ],
    },
    {
      category: 'Common use-cases',
      description:
        'Implement common use-cases for Actyx. including dashboards, ERP integrations, control logic and tool connectivity.',
      contents: [
        {
          title: 'Show machine data on a dashboard',
          link: '',
        },
        {
          title: 'Display ERP orders on tablets',
          link: '',
        },
        {
          title: 'Control AGVs delivering materials',
          link: '',
        },
        {
          title: 'Parameterise an assembly tool',
          link: '',
        },
      ],
    },
  ]
  return howToGuides
}

const howToGuides = createHowTos()
export default howToGuides