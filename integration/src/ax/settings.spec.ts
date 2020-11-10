// eslint-disable-next-line @typescript-eslint/no-var-requires
const nodeSettingSchema = require('../../../../protocols/json-schema/os/node-settings.schema.json')

import { runOnEach } from '../runner/hosts'
import { stubNode, stubNodeActyxosUnreachable, stubNodeHostUnreachable } from '../stubs'
import fetch from 'node-fetch'

describe('ax settings', () => {
  describe('scopes', () => {
    test('return ERR_NODE_UNREACHABLE', async () => {
      const response = await stubNodeHostUnreachable.ax.Settings.Scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE', async () => {
      const response = await stubNodeActyxosUnreachable.ax.Settings.Scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ax scope', async () => {
      const responses = await stubNode.ax.Settings.Scopes()
      const responsesShape = { code: 'OK', result: ['com.actyx.os'] }
      expect(responses).toMatchObject(responsesShape)
    })
  })

  describe('schema', () => {
    test('return ERR_NODE_UNREACHABLE', async () => {
      const response = await stubNodeHostUnreachable.ax.Settings.Schema('com.actyx.os')
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE', async () => {
      const response = await stubNodeActyxosUnreachable.ax.Settings.Schema('com.actyx.os')
      expect(response).toMatchErrNodeUnreachable()
    })
    // TODO: SPO look more into this, but probably the schema returned from docker is not updated with the latest master
    test.skip('return valid ax schema for node with no apps', async () => {
      const response = await stubNode.ax.Settings.Schema('com.actyx.os')
      expect(response).toMatchCodeOk()
      expect(response).toMatchObject(nodeSettingSchema)
    })

    // TODO: enable this test later when we can compare with the latest schema
    // https://github.com/Actyx/Cosmos/pull/5446#discussion_r512061598
    test.skip('schema in docs is updated with cli schema', async () => {
      const urlSchema = 'https://developer.actyx.com/schemas/os/node-settings.schema.json'
      const response = await fetch(urlSchema)
      const schemaDocs = await response.json()

      const responses = await runOnEach([{}], false, (node) =>
        node.ax.Settings.Schema('com.actyx.os'),
      )

      const schemaCli = responses.find((x) => x.code === 'OK' && x.result)
      expect(schemaCli).toMatchObject(schemaDocs)
    })
  })
})
