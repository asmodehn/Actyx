import { ConnectivityStatus } from './types'

describe('connectivity status codes', () => {
  it('should decode FullyConnected', () => {
    const v = {
      status: 'FullyConnected',
      inCurrentStatusForMs: 100,
    }

    expect(ConnectivityStatus.decode(v).value).toEqual(v)
  })

  it('should decode PartiallyConnected with empty specials', () => {
    const v = {
      status: 'PartiallyConnected',
      inCurrentStatusForMs: 100,
      swarmConnectivityLevel: 70,
      eventsToRead: 5,
      eventsToSend: 6,
      specialsDisconnected: [],
    }

    expect(ConnectivityStatus.decode(v).value).toEqual(v)
  })

  it('should decode PartiallyConnected with filled specials', () => {
    const v = {
      status: 'PartiallyConnected',
      inCurrentStatusForMs: 100,
      swarmConnectivityLevel: 70,
      eventsToRead: 5,
      eventsToSend: 6,
      specialsDisconnected: ['some-source'],
    }

    expect(ConnectivityStatus.decode(v).value).toEqual(v)
  })

  it('should decode NotConnected', () => {
    const v = {
      status: 'NotConnected',
      inCurrentStatusForMs: 2000000,
      eventsToRead: 5,
      eventsToSend: 6,
    }

    expect(ConnectivityStatus.decode(v).value).toEqual(v)
  })
})
