---
title: Event Service
---

This is a reference page for the ActyxOS **Event API**.

The Event Service HTTP API provides local access to the [Event Service](/os/docs/event-service.html), allowing you to

- [get information about known offsets](#get-information-about-known-offsets),
- [query event streams](#query-event-streams),
- [subscribe to event streams](#subscribe-to-event-streams); and,
- [publish events](#publish-events).

It is reachable at the following base URI: `http://localhost:4454/api/v1/events`.

:::info Pretty printed JSON
JSON used in the examples below is pretty-printed. This is only to make it more readable here. In reality, the Event Service API does not return pretty-printed JSON.
:::

## Get information about known offsets

You can get information from the Event Service about known offsets, i.e. what the event service believes to be the last offset for each stream.

### Request

- Endpoint: `http://localhost:4454/api/v1/events/offsets`
- HTTP method: `GET`
- HTTP headers:
    - `Content-Type`, must be `application/json`, default: `application/json`
    - (optional) `Accept`, must be `application/json`, default: `application/json`

There is no request body.

### Response

- HTTP headers:
    - `Content-Type` is `application/json`
    - `Cache-Control` is `no-store` (to get fresh data and not use cache slots)

The response body will contain a JSON object of the following structure:

```js
{ 
    "<string: sourceID>": "<integer: last-known-offset>",
    "<string: sourceID>": "<integer: last-known-offset>"
}
```

### Example

See the following example using cURL:

```bash
curl \
    -s -X "GET" \
    -H "Accept: application/json" \
    http://localhost:4454/api/v1/events/offsets | jq .
>
{ 
    "db66a77f": 57,
    "a263bad7": 60
}
```

## Query event streams

You can query the Event Service for bounded sets of events in one or more event streams.

### Request

- Endpoint: `http://localhost:4454/api/v1/events/query`
- HTTP method: `POST`
- HTTP headers:
    - `Content-Type`, must be `application/json`, default: `application/json`
    - (optional) `Accept`, must be `application/x-ndjson`, default: `application/x-ndjson`

The request body must contain a JSON object with the following structure:

```js
{
    "lowerBound": {
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. 34>",
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. -1>"
    },
    "upperBound": {
        "<string: sourceID>": "<integer: inclusive-upper-bound, e.g. 49>",
        "<string: sourceID>": "<integer: inclusive-upper-bound, e.g. 101>"
    },
    "subscriptions": [
        { 
            "semantics": "<string: semantics | undefined>",
            "name": "<string: name | undefined>",
            "source": "<string: sourceID> | undefined"
        },
        {
            "semantics": "<string: semantics | undefined>",
            "name": "<string: name | undefined>",
            "source": "<string: sourceID> | undefined"
        },
        {
            "semantics": "<string: semantics | undefined>",
            "name": "<string: name | undefined>",
            "source": "<string: sourceID> | undefined"
        }
    ],
    "order": "<string: 'lamport' | 'lamport-reverse' | 'source-ordered'"
}
```

You use the request body to specify the details of your request as documented in the following.

#### Optional: Lower bound for offsets (`lowerBound`)

The `lowerBound` object specifies the lower bound offset for each source id with the numbers being **exclusive**. i.e. a `lowerBound` specification of `34` means the event service will return events with offsets `> 34`.

The `lowerBound` is optional. If none is set for one, multiple or all subscribed sources, the Event Store will assume a lower bound offset of `-1`, i.e. the beginning.

#### Required: Upper bounds for offsets (`upperBound`)

The `upperBound` object specifies the upper bound offset for each source id with the numbers being **inclusive**. i.e. an `upperBound` specification of `34` means the event service will return events with offsets `<= 34`.

The `upperBound` is **required.** For every subscribed source where no upper bound offset it set, the result will be empty.

#### Required: Subscriptions (`subscriptions`)

The `subscriptions` object specifies which event streams should be queried, with streams being specified with the source, semantics and name 3-tuple. You may not provide some or all of these properties to specify wildcard.

Not specifying the source of a stream does not make sense in this context since no events will be returned for sources without a defined upper bound.

#### Required: Ordering (`order`)

The `order` object specifies in which order the events should be returned to the caller. There are three options, one of which must be specified:

1. `lamport`: ascending order according to events' [lamport timestamp](https://en.wikipedia.org/wiki/Lamport_timestamps)
2. `lamport-reverse`: descending order according to events' lamport timestamp
3. `source-ordered`: ascending order according to events' lamport timestamp per source, with no inter-source ordering guarantees

### Response

- HTTP headers:
    - `Content-Type` is `application/x-ndjson`
    - `Transfer-Encoding` is `chunked`

The response will be a stream of `<CR><LF>`-delimited event payloads of the following structure:

```js
{
    "stream": {
        "semantics": "<string: semantics>",
        "name": "<string: name>",
        "source": "<string: sourceID>"
    },
    "timestamp": "<integer>", // unix epoch in microseconds 
    "lamport": "<integer>",
    "offset": "<integer>",
    "payload": "<object>"
}
```

If an error is encountered while processing the stream of events, the stream will terminate with a final error JSON object with the following structure:

```js
{
    "error": "message",
    "errorCode": 500
}
```

### Example

See the following example using cURL:

```bash
echo '
{
    "lowerBound": {
        "db66a77f": 34,
        "a263bad7": -1
    },
    "upperBound": {
        "db66a77f": 57,
        "a263bad7": 60
    },
    "subscriptions": [
        {
            "semantics": "com.actyx.examples.temperature",
            "name": "temp-sensor",
            "source": "db66a77f"
        },
        {
            "semantics": "com.actyx.examples.temperature",
            "name": "temperatureSensor1",
            "source": "a263bad7"
        },
        {
            "name": "temperatureSensor2",
            "source": "a263bad7"
        }
    ],
    "order": "lamport-reverse"
}
'\
| curl \
    -X "POST" \
    -d @- \
    -H "Content-Type: application/json" \
    -H "Accept: application/x-ndjson" \
    http://localhost:4454/api/v1/events/query
    | jq
> {
    "stream": {
        "semantics": "com.actyx.examples.temperature",
        "name": "temp-sensor",
        "source": "db66a77f"
    },
    "timestamp": 21323,
    "lamport": 323,
    "offset": 34,
    "payload": {
        "foo": "bar",
        "fooArr": ["bar1", "bar2"]
    }
}
```

## Subscribe to event streams

You can use the Event Service API to subscribe to event streams. The Event Service may return past events and will return new events as they are received.

### Request

- Endpoint: `http://localhost:4454/api/v1/events/subscribe`
- HTTP method: `POST`
- HTTP headers:
    - `Content-Type`, must be `application/json`, default: `application/json`
    - (optional) `Accept`, must be `application/x-ndjson`, default: `application/x-ndjson`

The request body must contain a JSON object with the following structure:

```js
{
    "lowerBound": {
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. 34>",
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. -1>"
    },
    "subscriptions": [
        {
            "semantics": "<string: semantics | undefined>",
            "name": "<string: name | undefined>",
            "source": "<string: sourceID | undefined>"
        },
        {
            "semantics": "<string: semantics | undefined>",
            "name": "<string: name | undefined>",
            "source": "<string: sourceID | undefined>"
        },
        {
            "semantics": "<string: semantics | undefined>",
            "name": "<string: name | undefined>",
            "source": "<string: sourceID> | undefined"
        }
    ]
}
```

You use the request body to specify the details of your request as documented in the following.

#### Optional: Lower bound for offsets (`lowerBound`)

The `lowerBound` object specifies the lower bound offset for each source id with the numbers being **exclusive**. i.e. a `lowerBound` specification of `34` means the event service will return events with offsets `> 34`.

The `lowerBound` is optional. If none is set for one, multiple or all subscribed sources, the Event Store will assume a lower bound offset of `-1`, i.e. the beginning.

#### Required: Subscriptions (`subscriptions`)

The `subscriptions` objects specifies which event streams should be queried, with streams being specified with the source, semantics and name 3-tuple. You may not provide some or all of these properties to specify wildcard.

### Response

- HTTP headers:
    - `Content-Type` is `application/x-ndjson`
    - `Transfer-Encoding` is `chunked`

The response will be a stream of `<CR><LF>`-delimited event payloads of the following structure:

```js
{
    "stream": {
        "semantics": "<string: semantics>",
        "name": "<string: name>",
        "source": "<string: sourceID>"
    },
    "timestamp": "<integer>",
    "lamport": "<integer>",
    "offset": "<integer>",
    "payload": "<object>"
}
```

If an error is encountered while processing the stream of events, the stream will terminate with a final error JSON object with the following structure:

```js
{
    "error": "message",
    "errorCode": 500
}
```

### Example

See the following example using cURL:

```bash
echo '
{
    "lowerBound": {
            "db66a77f": 34,
        "a263bad7": -1
    },
    "subscriptions": [
        {
            "semantics": "com.actyx.examples.temperature",
            "name": "temp-sensor",
            "source": "db66a77f"
        },
        {
            "semantics": "com.actyx.examples.temperature",
            "name": "temperatureSensor1",
            "source": "a263bad7"
        },
        {
            "name": "temperatureSensor2",
            "source": "a263bad7"
        },
        {}
    ]
}
'\
| curl \
    -s -X "POST" \
    -d @- \
    -H "Content-Type: application/json" \
    -H "Accept: application/x-ndjson" \
    http://localhost:4454/api/v1/events/subscribe \
| jq . \
>
{
    "stream": {
        "semantics": "com.actyx.examples.temperature",
        "name": "temp-sensor",
        "source": "db66a77f"
    },
    "timestamp": 21323, // unix epoch microseconds 
    "lamport": 323,
    "offset": 34,
    "payload": {
        "foo": "bar",
        "fooArr": ["bar1", "bar2"]
    }
}
```

## Publish events

You can publish new events using the Event Service API.

### Request

- Endpoint: `http://localhost:4454/api/v1/events/publish`
- HTTP method: `POST`
- HTTP headers:
    - `Content-Type`, must be `application/json`, default: `application/json`

The request body must contain a JSON object with the following structure:

```js
{
    "data": [
        {
            "semantics": "<string: semantics>",
            "name": "<string: name>",
            "payload": "<object>"
        },
        {
            "semantics": "<string: semantics>",
            "name": "<string: name>",
            "payload": "<object>"
        }
    ]
}
```

You use the request body to provide the Event Service with the stream semantics, stream name and payload of the events to be published.

### Response

The response will provide feedback using HTTP status codes, with `201` signifying that the request was successfully processed and the events published.

### Example

See the following example using cURL:

```bash
echo '{
    "data": [
        {
            "semantics": "com.actyx.examples.temperature",
            "name": "temp-sensor-1",
            "payload": {
                "foo": [1, 3, 4],
                "bar": { "a": 1, "b": 103 }
        }
        },
        {
            "semantics": "com.actyx.examples.temperature",
            "name": "temp-sensor-2",
            "payload": {
                "foo": [3, 1, 1],
                "bar": { "a": 13, "b": 48 }
        }
        }
    ]
}
'\
| curl \
    -X "POST" \
    -d @- \
    -H "Content-Type: application/json" \
    http://localhost:4454/api/v1/events/publish
> Response: HTTP 201 | 500 | 400 with an invalid body
```

## Usage examples in different languages

The following examples show how you could interact with the event services from different languages and environments.

- [JavaScript (Node.js)](#javascript-nodejs)
- [JavaScript (browser)](#javascript-browser)
- [C&#35](#csharp)

### JavaScript (Node.js)

```js
const { Transform } = require('stream');
const http = require('http')
const StringDecoder = require('string_decoder').StringDecoder;

const decoder = new StringDecoder('utf8')

const actyxDecoder = new Transform({
    readableObjectMode: true,
    transform(chunk, _, cb) {
        try {
            if (this._last === undefined) { this._last = "" }
            this._last += decoder.write(chunk);
            var list = this._last.split(/\r?\n/);
            this._last = list.pop();
            for (var i = 0; i < list.length; i++) {
                if (list[i].length !== 0) { //ignore keep alive empty lines
                    const message = JSON.parse(list[i])
                    if (message.error !== undefined) {
                        return cb(message)
                    }
                    this.push(message);
                }
            }
            cb();
        } catch (err) {
            cb(err)
        }
    }
});

const options = {
    hostname: 'localhost',
    port: 4454,
    path: '/api/v1/events/subscribe',
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
}
const body = JSON.stringify({ subscriptions: [{ semantics: 'com.actyx.examples.temperature' }] })
const req = http.request(options, res => {

    if (res.statusCode == 200) {
        res.pipe(actyxDecoder).on('data', console.log).on("error", console.log)
    } else {
        console.log(`error, status code: ${res.statusCode}`)
    }
})
req.write(body);
req.end();
```

### JavaScript (browser)
```js
function ActyxDecoder(bodyStream) {
  return new ReadableStream({
    start(controller) {
      
      const dec = new TextDecoder()
      let last = ""
      // The following function handles each data chunk
      function push() {
        // "done" is a Boolean and value a "Uint8Array"
        bodyStream.read().then(({ done, value }) => {
          // Is there no more data to read?
          last += dec.decode(value)
          const list = last.split(/\r?\n/)
          last = list.pop();
          for (var i = 0; i < list.length; i++) {
            if (list[i].length !== 0) { //ignore keep alive empty lines
              const message = JSON.parse(list[i])
              if (message.error !== undefined) {
                return controller.error(message)
              }
              controller.enqueue(message)
            }
          }
          if (done) {
            // Tell the browser that we have finished sending data
            controller.close();
            return;
          }

          setTimeout(push, 0)
        });
      };

      push();
    }
  });
}
fetch('http://localhost:4454/api/v1/events/subscribe', {
    method: 'POST',
    body: JSON.stringify({ subscriptions: [{ semantics: 'com.actyx.examples.temperature' }] }),
    headers: { 'Content-Type': 'application/json' },
  })
  .then(r => r.body.getReader())
  .then(reader => {
    const axReader = ActyxDecoder(reader).getReader()
    const loop = () => {
      axReader.read().then(
        chunk => {
          if (!chunk.done) {
            console.log('event:', chunk.value)
            setTimeout(loop,0)
          } else {
            console.log('complete')
          }
        },
        error => {
          console.log('error:', error)
        },
      )
    }
    loop()
  })
```

### C&#35;

```csharp
using System;
using System.IO;
using System.Net;
using System.Text;
using System.Collections.Generic;

public class Application
{
    public static void Main()
    {
        var request = (HttpWebRequest)WebRequest.Create(new Uri("http://localhost:4454/api/v1/events/subscribe"));
        request.AllowReadStreamBuffering = false;
        var reqBody = Encoding.UTF8.GetBytes("{ subscriptions: [{ semantics: 'com.actyx.examples.temperature' }] }");
        request.Method = "POST";
        request.ContentType = "application/json";
        request.ContentLength = reqBody.Length;
        using (var stream = request.GetRequestStream())
        {
            stream.Write(reqBody, 0, reqBody.Length);
        }
        var response = request.GetResponse();
        using (var reader = new StreamReader(response.GetResponseStream(), Encoding.UTF8))
        {
            var delimiter = new char[] { '\r', '\n', '\r', '\n' };
            var charBuffer = new char[1024];
            int read;
            int partial = 0;
            do
            {
                read = reader.Read(charBuffer, partial, charBuffer.Length - partial);
                var split = Split(charBuffer, 0, partial + read, delimiter);
                var chunks = new ArraySegment<ArraySegment<char>>(split, 0, split.Length - 1);
                foreach (ArraySegment<char> chunk in chunks)
                {
                    Console.WriteLine("event: " + new string(chunk.ToArray()));
                }
                var rest = split[split.Length - 1];
                partial = rest.Count;
                if (partial > 0)
                    Array.Copy(rest.ToArray(), 0, charBuffer, 0, rest.Count);
            } while (read > -1);
        }
        Console.WriteLine("complete");
    }

    private static ArraySegment<char>[] Split(char[] arr, int startIndex, int length, char[] delimiter)
    {
        var result = new List<ArraySegment<char>>();
        var segStart = 0;
        for (int i = startIndex, j = 0; i < length; i++)
        {
            if (arr[i] != delimiter[j]) continue;
            if (j++ != delimiter.Length - 1) continue;
            var segLen = i - segStart - (delimiter.Length - 1);
            if (segLen > 0) result.Add(new ArraySegment<char>(arr, segStart, segLen));
            segStart = i + 1;
            j = 0;
        }

        result.Add(new ArraySegment<char>(arr, segStart, length - segStart));
        return result.ToArray();
    }
}
```