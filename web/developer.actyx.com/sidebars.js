module.exports = {
  howToSidebar: [
    {
      type: 'category',
      label: 'How-to Guides',
      collapsed: false,
      items: ['how-to-guides/overview'],
    },
    {
      type: 'category',
      label: 'Local Development',
      collapsed: true,
      items: [
        'how-to-guides/local-development/installing-actyx',
        'how-to-guides/local-development/starting-a-new-project',
        'how-to-guides/local-development/setting-up-your-environment',
        'how-to-guides/local-development/installing-cli-node-manager',
        // 'how-to-guides/local-development/obtaining-a-development-certificate',
        'how-to-guides/local-development/common-development-errors',
      ],
    },
    {
      type: 'category',
      label: 'Process Logic',
      collapsed: true,
      items: [
        'how-to-guides/process-logic/publishing-to-event-streams',
        'how-to-guides/process-logic/subscribing-to-event-streams',
        'how-to-guides/process-logic/computing-states-from-events',
        'how-to-guides/process-logic/automating-decision-making',
        'how-to-guides/process-logic/dealing-with-network-partitions',
        'how-to-guides/process-logic/modelling-processes-in-twins',
        'how-to-guides/process-logic/transferring-twins-into-code',
      ],
    },
    {
      type: 'category',
      label: 'Actyx SDK',
      collapsed: true,
      items: ['how-to-guides/sdk/placeholder'],
    },
    {
      type: 'category',
      label: 'Data import and export',
      collapsed: true,
      items: [
        'how-to-guides/integrating-with-actyx/user-interface',
        'how-to-guides/integrating-with-actyx/other-software',
        'how-to-guides/integrating-with-actyx/front-end-frameworks',
        'how-to-guides/integrating-with-actyx/plcs',
        'how-to-guides/integrating-with-actyx/erps',
        'how-to-guides/integrating-with-actyx/bi-analytics',
      ],
    },
    {
      type: 'category',
      label: 'Testing',
      collapsed: true,
      items: [
        'how-to-guides/testing/testing-pipeline',
        'how-to-guides/testing/unit-testing-with-jest',
        'how-to-guides/testing/unit-testing-with-cypress',
        'how-to-guides/testing/integration-testing',
        'how-to-guides/testing/ci-cd-pipeline',
      ],
    },
    {
      type: 'category',
      label: 'Packaging & Deployment',
      collapsed: true,
      items: [
        'how-to-guides/configuring-and-packaging/front-end-apps',
        'how-to-guides/configuring-and-packaging/headless-apps',
        'how-to-guides/configuring-and-packaging/deployment-to-production',
        'how-to-guides/configuring-and-packaging/updating-a-solution',
        'how-to-guides/configuring-and-packaging/actyx-swarms',
        'how-to-guides/configuring-and-packaging/bootstrap-node',
      ],
    },
    {
      type: 'category',
      label: 'Monitoring & Debugging',
      collapsed: true,
      items: [
        'how-to-guides/monitoring-debugging/node-logs',
        'how-to-guides/monitoring-debugging/app-logs',
        'how-to-guides/monitoring-debugging/connectivity-status',
        'how-to-guides/monitoring-debugging/mobile-device-management',
        'how-to-guides/monitoring-debugging/bash',
      ],
    },
    {
      type: 'category',
      label: 'Common Use-Cases',
      collapsed: true,
      items: [
        'how-to-guides/common-use-cases/showing-data-on-a-dashboard',
        'how-to-guides/common-use-cases/erp-orders-on-tablets',
        'how-to-guides/common-use-cases/controlling-agvs',
        'how-to-guides/common-use-cases/parameterise-assembly-tool',
      ],
    },
  ],
  conceptualSidebar: [
    {
      type: 'category',
      label: 'Conceptual Guides',
      collapsed: false,
      items: ['conceptual-guides/overview'],
    },
    {
      type: 'category',
      label: 'Contents',
      collapsed: false,
      items: [
        'conceptual-guides/event-based-systems',
        'conceptual-guides/distributed-system-architectures',
        'conceptual-guides/local-first-cooperation',
        // 'conceptual-guides/thinking-in-actyx',
        'conceptual-guides/actyx-jargon',
        // 'conceptual-guides/actyx-vs-the-cloud',
        'conceptual-guides/peer-discovery',
        'conceptual-guides/performance-and-limits-of-actyx',
        'conceptual-guides/security-in-actyx',
        'conceptual-guides/the-actyx-node',
        'conceptual-guides/actyx-node-lifecycle',
        // 'conceptual-guides/apps-in-the-factory-context',
      ],
    },
  ],
  referenceSidebar: [
    {
      type: 'category',
      label: 'Reference Documentation',
      collapsed: false,
      items: ['reference/overview'],
    },
    {
      type: 'category',
      label: 'Contents',
      collapsed: false,
      items: [
        'reference/actyx-reference',
        'reference/event-service',
        'reference/pond-api-reference',
        'reference/js-ts-sdk',
        'reference/rust-sdk',
        'reference/cli',
        'reference/node-manager',
      ],
    },
  ],
  faqSidebar: [
    {
      type: 'category',
      label: 'FAQ',
      collapsed: false,
      items: [
        'faq/supported-programming-languages',
        'faq/supported-edge-devices',
        'faq/supported-device-operating-systems',
        'faq/integrating-with-machines',
        'faq/integrating-with-software-systems',
        'faq/pre-built-actyxos-apps',
        'faq/network-requirements',
        'faq/latency-and-performance',
        'faq/number-of-devices',
        'faq/running-out-of-disk-space',
      ],
    },
  ],
}
