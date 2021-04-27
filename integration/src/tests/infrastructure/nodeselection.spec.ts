import { selectNodes } from '../../infrastructure/nodeselection'
import { ActyxOSNode } from '../../infrastructure/types'
import { mkNodeStub } from '../../stubs'

let n1: ActyxOSNode
let n2: ActyxOSNode
let n3: ActyxOSNode
beforeAll(async () => {
  n1 = await mkNodeStub('android', 'aarch64', 'android', 'n0')
  n2 = await mkNodeStub('linux', 'x86_64', 'docker', 'n1')
  n3 = await mkNodeStub('windows', 'aarch64', 'process', 'n2')
})

describe('NodeSelection', () => {
  it('should fail', () => {
    expect(selectNodes([{ os: 'linux' }], [])).toEqual(null)
  })
  it('should select single node', () => {
    expect(selectNodes([{ os: 'linux' }], [n1, n2, n3])).toEqual([n2])
  })
  it('should select multiple', () => {
    expect(selectNodes([{}, {}, {}], [n1, n2, n3])).toEqual([n1, n2, n3])
    expect(selectNodes([{}, {}, { host: 'process' }], [n1, n2, n3])).toEqual([n1, n2, n3])
  })
})