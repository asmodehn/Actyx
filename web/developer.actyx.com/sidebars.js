module.exports = {
  homeSidebar: ["home/actyx_platform", "home/actyx_products"],
  osSidebar: [
    {
      type: "category",
      label: "General",
      collapsed: false,
      items: ["os/general/introduction", "os/general/installation"],
    },
    {
      type: "category",
      label: "Guides",
      collapsed: false,
      items: [
        "os/guides/overview",
        "os/guides/swarms",
        "os/guides/building-apps",
        "os/guides/running-apps",
        "os/guides/event-streams",
      ],
    },
    {
      type: "category",
      label: "Advanced\u00a0Guides",
      collapsed: false,
      items: [
        "os/advanced-guides/overview",
        "os/advanced-guides/actyxos-on-android",
        "os/advanced-guides/actyxos-on-docker",
        "os/advanced-guides/actyxos-on-windows",
        "os/advanced-guides/actyxos-on-linux",
        "os/advanced-guides/actyxos-on-macos",
        "os/advanced-guides/node-and-app-lifecycle",
        "os/advanced-guides/node-and-app-settings",
        "os/advanced-guides/actyxos-bootstrap-node",
        "os/advanced-guides/using-vscode-for-schema-validation",
        "os/advanced-guides/provided-security",
      ],
    },
    {
      type: "category",
      label: "API\u00a0Reference",
      collapsed: false,
      items: [
        "os/api/overview",
        "os/api/event-service",
        "os/api/blob-service",
        "os/api/console-service",
        "os/api/node-settings-schema",
        "os/api/app-manifest-schema",
      ],
    },
    {
      type: "category",
      label: "SDKs",
      collapsed: false,
      items: [
        "os/sdks/overview",
        "os/sdks/rust",
        "os/sdks/js-ts",
      ],
    },
    {
      type: "category",
      label: "Theoretical\u00a0Foundation",
      collapsed: false,
      items: [
        "os/theoretical-foundation/distributed-systems",
        "os/theoretical-foundation/event-sourcing",
        "os/theoretical-foundation/actyxos-and-cap",
      ],
    },
    "os/release-notes",
  ],
  pondv1Sidebar: {
    "Versions\u00a0(current:\u00a0v1)": [
      { type: "link", href: "/docs/pond/introduction", label: "v2" },
    ],
    "Actyx Pond": ["pond-v1/getting-started", "pond-v1/programming-model"],
    Guides: [
      "pond-v1/guides/hello-world",
      "pond-v1/guides/events",
      "pond-v1/guides/local-state",
      "pond-v1/guides/subscriptions",
      "pond-v1/guides/time-travel",
      "pond-v1/guides/commands",
      "pond-v1/guides/types",
      "pond-v1/guides/snapshots",
      "pond-v1/guides/integrating-a-ui",
    ],
  },
  pondSidebar: [
    {
      type: "category",
      label: "Versions\u00a0(current:\u00a0v2)",
      items: [
        { type: "link", href: "/docs/pond-v1/getting-started", label: "v1" },
      ],
    },
    "pond/introduction",
    "pond/getting-started",
    "pond/api-reference",
    "pond/exception-handling",
    {
      type: "category",
      label: "Learning\u00a0the\u00a0Pond\u00a0in\u00a010\u00a0steps",
      collapsed: false,
      items: [
        "pond/guides/hello-world",
        "pond/guides/events",
        "pond/guides/local-state",
        "pond/guides/subscriptions",
        "pond/guides/typed-tags",
        "pond/guides/time-travel",
        "pond/guides/state-effects",
        "pond/guides/types",
        "pond/guides/snapshots",
        "pond/guides/integrating-a-ui",
      ],
    },
    {
    type: 'category',
    label: 'Fish\u00a0Parameters',
    collapsed: false,
    items: [
      'pond/fish-parameters/on-event',
      'pond/fish-parameters/initial-state',
      'pond/fish-parameters/where',
      'pond/fish-parameters/fish-id',
      'pond/fish-parameters/deserialize-state',
      'pond/fish-parameters/is-reset',
      ]
    },
    {
      type: "category",
      label: "Pond In-Depth",
      collapsed: false,
      items: [
        "pond/in-depth/tag-type-checking",
        "pond/in-depth/eventual-consistency",
        "pond/in-depth/do-not-ignore-events",
	"pond/in-depth/cycling-states",
	"pond/in-depth/observe-all",
	"pond/in-depth/observe-one",
      ],
    },
    "pond/pond-extensions",
  ],
  nodeManagerSidebar: [
    "node-manager/overview",
    "node-manager/functionality",
  ],
  cliSidebar: [
    "cli/getting-started",
    "cli/ax",
    {
      type: "category",
      label: "ax\u00a0nodes",
      collapsed: false,
      items: ["cli/nodes/nodes", "cli/nodes/ls"],
    },
    {
      type: "category",
      label: "ax\u00a0apps",
      collapsed: false,
      items: [
        "cli/apps/apps",
        "cli/apps/ls",
        "cli/apps/validate",
        "cli/apps/package",
        "cli/apps/deploy",
        "cli/apps/undeploy",
        "cli/apps/start",
        "cli/apps/stop",
      ],
    },
    {
      type: "category",
      label: "ax\u00a0settings",
      collapsed: false,
      items: [
        "cli/settings/settings",
        "cli/settings/scopes",
        "cli/settings/schema",
        "cli/settings/get",
        "cli/settings/set",
        "cli/settings/unset",
      ],
    },
    {
      type: "category",
      label: "ax\u00a0logs",
      collapsed: false,
      items: ["cli/logs/logs", "cli/logs/tail"],
    },
    {
      type: "category",
      label: "ax\u00a0swarms",
      collapsed: false,
      items: ["cli/swarms/swarms", "cli/swarms/keygen"],
    },
    "cli/release-notes",
  ],
  learnActyxSidebar: [
    "learn-actyx",
    "learn-actyx/quickstart",
    "learn-actyx/tutorial",
    {
      type: "category",
      label: "Advanced\u00a0Tutorial",
      collapsed: false,
      items: [
        "learn-actyx/advanced-tutorial/introduction",
        "learn-actyx/advanced-tutorial/solution-architecture",
        "learn-actyx/advanced-tutorial/get-started",
        "learn-actyx/advanced-tutorial/explore-the-apps",
        "learn-actyx/advanced-tutorial/next-steps",
      ],
    },
  ],
  faqSidebar: [
    "faq/supported-programming-languages",
    "faq/supported-edge-devices",
    "faq/supported-device-operating-systems",
    "faq/integrating-with-machines",
    "faq/integrating-with-software-systems",
    "faq/pre-built-actyxos-apps",
    "faq/network-requirements",
    "faq/latency-and-performance",
    "faq/number-of-devices",
    "faq/running-out-of-disk-space",
  ],
};
