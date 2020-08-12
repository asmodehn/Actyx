using System;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System.Net.Http;
using System.Text;
using System.IO;
using System.Net;
using System.Net.Mime;
using System.Threading.Tasks;
using System.Net.Http.Headers;
using System.Collections.Generic;
using System.Threading;

namespace Actyx {

    public class StreamingResponse<T> : IAsyncEnumerator<T> {

	private readonly StreamReader reader;

	public StreamingResponse(Stream responseDataStream) {
	    this.reader = new StreamReader(responseDataStream);
	}

	public T Current { get; private set; }

	public async ValueTask<bool> MoveNextAsync() {
	    if (reader.EndOfStream) {
		return false;
	    }

	    string nextLine = await reader.ReadLineAsync();

	    // Empty lines are sent as a means of keep-alive.
	    while (nextLine != "event:event") {
		Console.WriteLine("skipping: " + nextLine);
		nextLine = await reader.ReadLineAsync();
	    }

	    // Immediately after the event:event line we expect the data:{json} line
	    nextLine = await reader.ReadLineAsync();
	    while (!nextLine.StartsWith("data:")) {
		Console.WriteLine("EXPECTED DATA BUT FOUND: " + nextLine);
		nextLine = await reader.ReadLineAsync();
	    }

	    // Drop the "data:" prefix and deserialize
	    string jsonData = nextLine.Substring(5);
	    this.Current = JsonConvert.DeserializeObject<T>(jsonData);

	    return true;
	}

	public async ValueTask DisposeAsync() {
	    reader.Dispose();
	}
    }


    public class ActyxRequest<T> : IAsyncEnumerable<T> {
	private readonly WebRequest request;

	public ActyxRequest(WebRequest request) {
	    this.request = request;
	}

	public IAsyncEnumerator<T> GetAsyncEnumerator(CancellationToken token)
	{    
	    return new StreamingResponse<T>(request.GetResponse().GetResponseStream());
	}
    }

    public class EventService
    {
	private readonly string authToken;
	private readonly string endpoint;

	public static async Task<EventService> ForApp(
			    string appName,
			    string endpoint = "http://localhost",
			    int eventServicePort = 4454,
			    int nodePort = 4457
			    )
	{
	    var request = WebRequest.Create(endpoint + ':' + nodePort + "/api/v0/apps/" + Uri.EscapeUriString(appName) + "/token");

	    var response = await request.GetResponseAsync();

	    var reader = new StreamReader(response.GetResponseStream());

	    string token = "Bearer " + JObject.Parse(reader.ReadLine())["Ok"].ToObject<string>();

	    Console.WriteLine("found token: " + token);

	    return new EventService(token, endpoint, eventServicePort);
	}

	public EventService(
			    string authToken,
			    string endpoint = "http://localhost",
			    int eventServicePort = 4454
			    )
	{
	    this.authToken = authToken;
	    this.endpoint = endpoint + ':' + eventServicePort;
	}

	private WebRequest EventServiceRequest(string path)
	{
	    WebRequest request = WebRequest.Create(this.endpoint + path);
	    request.ContentType = "application/json";
	    request.Headers.Add("Authorization", this.authToken);

	    return request;
	}

	private WebRequest Post(string path, string postData)
	{
	    WebRequest request = this.EventServiceRequest(path);
	    // Setup POST data:
	    request.Method = "POST";
	    byte[] reqMsgBytes = Encoding.UTF8.GetBytes(postData);

	    Stream dataStream = request.GetRequestStream();
	    dataStream.Write(reqMsgBytes, 0, reqMsgBytes.Length);
	    dataStream.Close();
	    
	    return request;
	}


	public async Task<Dictionary<string, UInt64>> offsets()
	{
	    var request = this.EventServiceRequest(this.endpoint + "/api/v2/events/offsets");
	    
	    var response = await request.GetResponseAsync();

	    var reader = new StreamReader(response.GetResponseStream());

	    return JsonConvert.DeserializeObject<Dictionary<string, UInt64>>(reader.ReadLine());
	}
	

	public IAsyncEnumerable<ISuttMessage> subscribeUntilTimeTravel(string session, string subscription, IDictionary<string, UInt64> offsets)
	{
	    var req = new {
		session,
		subscription,
		offsets
	    };

	    string postData = JsonConvert.SerializeObject(req);

	    return new ActyxRequest<ISuttMessage>(this.Post("/api/v2/events/subscribeUntilTimeTravel", postData));
	}


	public IAsyncEnumerable<ISuttMessage> subscribeUntilTimeTravel(string session, string subscription, params SnapshotCompression[] acceptedFormats)
	{
	    List<string> compression = new List<string>();

	    if (acceptedFormats.Length == 0) {
		compression.Add(SnapshotCompression.None.ToString());
	    } else {
		foreach (var accepted in acceptedFormats) {
		    compression.Add(accepted.ToString().ToLower());
		}
	    }

	    var req = new {
		session,
		subscription,
		snapshot = new {
		    compression
		}
	    };

	    string postData = JsonConvert.SerializeObject(req);
	    Console.WriteLine("posting:" + postData);

	    return new ActyxRequest<ISuttMessage>(this.Post("/api/v2/events/subscribeUntilTimeTravel", postData));
	}

	public IAsyncEnumerable<Event> subscribe()
	{
	    var req = new {
		subscriptions = new List<object>() {
		    new {
			semantics = "whatever",
		    },
		}
	    };

	    // string postData = "{\"subscriptions\": [{}]}";
	    string postData = JsonConvert.SerializeObject(req);

	    return new ActyxRequest<Event>(this.Post("/api/v2/events/subscribe", postData));
	}
    }
}
