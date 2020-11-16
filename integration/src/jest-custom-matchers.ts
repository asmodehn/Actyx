/* eslint-disable @typescript-eslint/no-explicit-any */
const hasPropertyResponse = (response: any, propery: string, value?: string) => {
  if (value) {
    return response && propery in response && response[propery] === value
  } else {
    return response && propery in response
  }
}

const validateResponseError = (response: any, codeExpected: string) => {
  const hasMessage = hasPropertyResponse(response, 'message')
  const hasCode = hasPropertyResponse(response, 'code', codeExpected)
  const pass = hasCode && hasMessage
  const message = () =>
    `Expected code was: ${codeExpected} instead got: ${response.code}, message was ${
      hasMessage ? 'found' : 'not found'
    }, response was: ${JSON.stringify(response)}`
  return {
    message,
    pass,
  }
}

expect.extend({
  toMatchErrNodeUnreachable(response: any): jest.CustomMatcherResult {
    const { message, pass } = validateResponseError(response, 'ERR_NODE_UNREACHABLE')
    return {
      message,
      pass,
    }
  },
  toMatchErrInvalidInput(response: any): jest.CustomMatcherResult {
    const { message, pass } = validateResponseError(response, 'ERR_INVALID_INPUT')
    return {
      message,
      pass,
    }
  },
  toMatchCodeOk(response: any): jest.CustomMatcherResult {
    const expectedCode = 'OK'
    const pass = hasPropertyResponse(response, 'code', 'OK')
    const message = () =>
      `Expected code was: ${expectedCode}, instead got: ${
        response.code
      }, response was: ${JSON.stringify(response)}`
    return {
      message,
      pass,
    }
  },
})