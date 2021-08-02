﻿using System;
using System.Net.Http;
using System.Text;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Utils.Extensions;
using Newtonsoft.Json;

namespace Actyx.Sdk.AxHttpClient
{
    public class AxHttpClient : IAxHttpClient
    {
        private static readonly HttpClient httpClient;
        static AxHttpClient()
        {
            httpClient = new HttpClient();
        }


        public static async Task<NodeId> GetNodeId(Uri baseUri)
        {
            var nodeIdResponse = await httpClient.GetAsync(baseUri + HttpApiPath.NODE_ID_SEG);
            await nodeIdResponse.EnsureSuccessStatusCodeCustom();
            var nodeId = await nodeIdResponse.Content.ReadAsStringAsync();
            return new NodeId(nodeId);
        }

        public static async Task<AxHttpClient> Create(string baseUrl, AppManifest manifest)
        {
            ThrowIf.Argument.IsNull(baseUrl, nameof(baseUrl));

            var client = new AxHttpClient(baseUrl, manifest)
            {
                NodeId = await GetNodeId(new Uri(baseUrl)),
            };
            client.token = (await GetToken(client.uriBuilder.Uri, manifest)).Token;

            return client;
        }

        private readonly UriBuilder uriBuilder;
        private readonly AppManifest manifest;
        private string token;

        public NodeId NodeId { get; private set; }

        public string AppId => manifest.AppId;

        private AxHttpClient(string baseUrl, AppManifest manifest)
        {
            this.manifest = manifest;
            if (!Uri.TryCreate(baseUrl, UriKind.Absolute, out Uri uri))
            {
                throw new ArgumentException($"Base url needs to be an absolute, i.e. 'http://localhost:4454'. Received '{baseUrl}'.");
            }
            if (!uri.Scheme.Equals("http"))
            {
                throw new ArgumentException($"Only http scheme allowed, i.e. 'http://localhost:4454'. Received '{baseUrl}'.");
            }
            uriBuilder = new UriBuilder(uri)
            {
                Path = HttpApiPath.API_V2_PATH,
            };
        }

        public Task<HttpResponseMessage> Post<T>(string path, T data, bool xndjson = false) =>
            FetchWithRetryOnUnauthorized(() =>
            {
                var request = new HttpRequestMessage(HttpMethod.Post, MkApiUrl(path));
                request.Headers.Add("Accept", xndjson ? "application/x-ndjson" : "application/json");
                request.Headers.Add("Authorization", $"Bearer {token}");
                request.Content = CreateJsonContent(data);
                return httpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead);
            });

        public Task<HttpResponseMessage> Get(string path) =>
            FetchWithRetryOnUnauthorized(() =>
            {
                var request = new HttpRequestMessage(HttpMethod.Get, MkApiUrl(path));
                request.Headers.Add("Authorization", $"Bearer {token}");
                request.Headers.Add("Accept", "application/json");
                return httpClient.SendAsync(request);
            });

        public static async Task<AuthenticationResponse> GetToken(Uri baseUri, AppManifest manifest)
        {
            var response = await httpClient.PostAsync(baseUri + HttpApiPath.AUTH_SEG, CreateJsonContent(manifest));
            await response.EnsureSuccessStatusCodeCustom();
            return await response.Content.ReadFromJsonAsync<AuthenticationResponse>();
        }

        private static StringContent CreateJsonContent<T>(T value)
        {
            var json = JsonConvert.SerializeObject(value, HttpContentExtensions.JsonSettings);
            return new StringContent(json, Encoding.UTF8, "application/json");
        }

        private string MkApiUrl(string path) => uriBuilder.Uri + path;

        private async Task<HttpResponseMessage> FetchWithRetryOnUnauthorized(Func<Task<HttpResponseMessage>> request)
        {
            var response = await request();
            if (response.IsUnauthorized())
            {
                token = (await GetToken(uriBuilder.Uri, manifest)).Token;
                response = await request();
            }

            return response;
        }
    }
}
