module.exports = {
  title: 'Actyx Developer',
  tagline: 'Documentation, guides and tools for building on the Actyx Platform',
  url: 'https://developer.actyx.com',
  baseUrl: '/',
  favicon: 'img/favicon.ico',
  organizationName: 'Actyx',
  projectName: 'Actyx Developer Website',
  onBrokenLinks: 'ignore',
  plugins: [
    [
      require.resolve('docusaurus-gtm-plugin'),
      {
        id: 'GTM-5PXCMFH',
      },
    ],
    [require.resolve('./src/plugins/analytics'), {}],
    [
      require.resolve('./src/plugins/arm'),
      {
        releasesYml: './__auto-releases.yml',
      },
    ],
  ],
  themeConfig: {
    announcementBar: {
      id: 'v2-release',
      content: 'Announcement Bar Content',
      backgroundColor: '#f5f6f7',
      textColor: '#000',
    },
    colorMode: {
      defaultMode: 'light',
      disableSwitch: true,
      respectPrefersColorScheme: false,
    },
    navbar: {
      title: '',
      logo: {
        alt: 'Actyx Developer',
        src: 'img/header.svg',
      },
      items: [
        {
          label: 'Actyx',
          activeBasePath: 'docs/hello',
          to: 'docs/hello',
        },
        {
          label: 'SDKs',
          activeBasePath: 'docs/hello',
          to: 'docs/hello',
        },
        {
          label: 'CLI',
          activeBasePath: 'docs/hello',
          to: 'docs/hello',
        },
        {
          label: 'Node Manager',
          activeBasePath: 'docs/hello',
          to: 'docs/hello',
        },
        {
          label: 'Building Apps',
          activeBasePath: 'docs/hello',
          to: 'docs/hello',
        },
      ],
    },
    footer: {
      logo: {
        alt: 'Actyx Developer',
        src: 'img/header.svg',
        href: 'https://developer.actyx.com',
      },
      style: 'light',
      links: [
        {
          title: 'Footer Header',
          items: [
            {
              label: 'Footer Label',
              to: 'docs/hello',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Actyx AG`,
    },
    prism: {
      theme: require('prism-react-renderer/themes/palenight'),
      darkTheme: require('prism-react-renderer/themes/dracula'),
      additionalLanguages: ['rust', 'csharp'],
    },
    algolia: {
      apiKey: 'dee14099c148f0ca14d046428003623a',
      indexName: 'actyx_developer',
      algoliaOptions: {}, // Optional, if provided by Algolia
    },
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],
}
