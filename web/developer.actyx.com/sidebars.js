module.exports = {
  howToSidebar: [
    'how-to/overview',
    {
      type: 'category',
      label: 'Local development',
      collapsed: true,
      items: [
        'how-to/local-development/install-actyx',
        'how-to/local-development/install-cli-node-manager',
        'how-to/local-development/reset-your-node',
      ],
    },
    {
      type: 'category',
      label: 'Business logic',
      collapsed: true,
      items: [
        'how-to/actyx-pond/introduction',
        'how-to/actyx-pond/getting-started',
        {
          type: 'category',
          label: 'Pond in 10 Steps',
          collapsed: true,
          items: [
            'how-to/actyx-pond/guides/hello-world',
            'how-to/actyx-pond/guides/events',
            'how-to/actyx-pond/guides/local-state',
            'how-to/actyx-pond/guides/subscriptions',
            'how-to/actyx-pond/guides/typed-tags',
            'how-to/actyx-pond/guides/time-travel',
            'how-to/actyx-pond/guides/state-effects',
            'how-to/actyx-pond/guides/types',
            'how-to/actyx-pond/guides/snapshots',
            'how-to/actyx-pond/guides/integrating-a-ui',
          ],
        },
        {
          type: 'category',
          label: 'Fish Parameters',
          collapsed: true,
          items: [
            'how-to/actyx-pond/fish-parameters/on-event',
            'how-to/actyx-pond/fish-parameters/initial-state',
            'how-to/actyx-pond/fish-parameters/where',
            'how-to/actyx-pond/fish-parameters/fish-id',
            'how-to/actyx-pond/fish-parameters/deserialize-state',
            'how-to/actyx-pond/fish-parameters/is-reset',
          ],
        },
        {
          type: 'category',
          label: 'Pond in-Depth',
          collapsed: true,
          items: [
            'how-to/actyx-pond/in-depth/tag-type-checking',
            'how-to/actyx-pond/in-depth/eventual-consistency',
            'how-to/actyx-pond/in-depth/do-not-ignore-events',
            'how-to/actyx-pond/in-depth/cycling-states',
            'how-to/actyx-pond/in-depth/observe-all',
            'how-to/actyx-pond/in-depth/observe-one',
            'how-to/actyx-pond/in-depth/exception-handling',
          ],
        },
        'how-to/actyx-pond/pond-extensions',
      ],
    },
    {
      type: 'category',
      label: 'Structured queries',
      collapsed: true,
      items: ['how-to/structured-queries/query-events-with-aql'],
    },
    {
      type: 'category',
      label: 'User Auth',
      collapsed: true,
      items: [
        'how-to/user-auth/set-up-user-keys',
        'how-to/user-auth/manage-authorized-users',
        'how-to/user-auth/get-developer-certificate',
      ],
    },
    {
      type: 'category',
      label: 'App Auth',
      collapsed: true,
      items: [
        'how-to/app-auth/sign-app-manifest',
        'how-to/app-auth/authenticate-with-app-manifest',
      ],
    },
    {
      type: 'category',
      label: 'Swarms',
      collapsed: true,
      items: [
        'how-to/swarms/setup-swarm',
        'how-to/swarms/connect-nodes',
        'how-to/swarms/configure-announced-addresses',
      ],
    },
    {
      type: 'category',
      label: 'Packaging',
      collapsed: true,
      items: [
        'how-to/packaging/mobile-apps',
        'how-to/packaging/desktop-apps',
        'how-to/packaging/headless-apps',
      ],
    },
    {
      type: 'category',
      label: 'Operations',
      collapsed: true,
      items: [
        'how-to/operations/device-management',
        'how-to/operations/discovery-helper-node',
        'how-to/operations/log-as-json',
        'how-to/operations/disable-colored-logs',
      ],
    },
    {
      type: 'category',
      label: 'Monitoring & Debugging',
      collapsed: true,
      items: [
        'how-to/monitoring-debugging/access-logs',
        'how-to/monitoring-debugging/logging-levels',
        'how-to/monitoring-debugging/network-requirements',
        'how-to/monitoring-debugging/node-connections',
      ],
    },
    {
      type: 'category',
      label: 'Licensing',
      collapsed: true,
      items: ['how-to/licensing/license-nodes', 'how-to/licensing/license-apps'],
    },
    {
      type: 'category',
      label: 'Troubleshooting',
      collapsed: true,
      items: [
        'how-to/troubleshooting/installation-and-startup',
        'how-to/troubleshooting/app-to-node-communication',
        'how-to/troubleshooting/node-to-cli-communication',
        'how-to/troubleshooting/node-synchronization',
      ],
    },
    {
      type: 'category',
      label: 'Migration from v1',
      collapsed: true,
      items: [
        'how-to/migration/migration-overview',
        {
          type: 'category',
          label: 'Apps',
          items: [
            'how-to/migration/migrate-business-logic',
            'how-to/migration/migrate-app-manifest',
            'how-to/migration/migrate-app-logs-and-settings',
          ],
        },
        {
          type: 'category',
          label: 'Nodes',
          items: [
            'how-to/migration/migrate-bootstrap-nodes',
            'how-to/migration/migrate-production-nodes',
            'how-to/migration/migrate-externally-stored-offset-maps',
          ],
        },
      ],
    },
  ],
  conceptualSidebar: [
    'conceptual/overview',
    'conceptual/how-actyx-works',
    'conceptual/event-streams',
    'conceptual/tags',
    'conceptual/actyx-jargon',
    'conceptual/discovery',
    'conceptual/performance-and-limits',
    'conceptual/authentication-and-authorization',
    'conceptual/operations',
    'conceptual/security',
    'conceptual/distributed-systems',
    'conceptual/event-sourcing',
  ],
  referenceSidebar: [
    'reference/overview',
    'reference/actyx-reference',
    {
      type: 'category',
      label: 'Actyx API',
      collapsed: true,
      items: [
        'reference/node-api',
        'reference/auth-api',
        'reference/events-api',
        'reference/files-api',
      ],
    },
    {
      type: 'category',
      label: 'Actyx Pond (JS/TS)',
      collapsed: true,
      items: [
        {
          //label: 'Actyx Pond (JS/TS)',
          type: 'autogenerated',
          dirName: 'reference/pond', // 'api' is the 'out' directory
        },
      ],
    },
    //{
    //  type: 'category',
    //  label: 'Actyx Pond (JS/TS)',
    //  collapsed: true,
    //  items: require('./__pond-sidebar'),
    //},
    'reference/node-manager',
    {
      type: 'category',
      label: 'Actyx CLI',
      collapsed: true,
      items: [
        'reference/cli/cli-overview',
        'reference/cli/apps/sign',
        'reference/cli/events/dump',
        'reference/cli/events/offsets',
        'reference/cli/events/publish',
        'reference/cli/events/query',
        'reference/cli/events/restore',
        'reference/cli/nodes/ls',
        'reference/cli/nodes/inspect',
        'reference/cli/settings/schema',
        'reference/cli/settings/get',
        'reference/cli/settings/set',
        'reference/cli/settings/unset',
        'reference/cli/swarms/keygen',
        'reference/cli/users/keygen',
        'reference/cli/users/add-key',
      ],
    },
    'reference/aql',
  ],
  tutorialSidebar: [
    {
      type: 'doc',
      id: 'tutorials/overview', // string - document id
    },
    {
      type: 'doc',
      id: 'tutorials/quickstart', // string - document id
    },
    {
      type: 'doc',
      id: 'tutorials/chat', // string - document id
    },
    { type: 'doc', id: 'tutorials/aql' },
  ],
}
