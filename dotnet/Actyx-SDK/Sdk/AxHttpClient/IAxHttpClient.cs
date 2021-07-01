﻿
using System.Net.Http;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;

namespace Actyx.Sdk.AxHttpClient
{
    public interface IAxHttpClient
    {
        NodeId NodeId { get; }

        string AppId { get; }

        Task<HttpResponseMessage> Post<T>(string path, T data, bool xndjson = false);
        Task<HttpResponseMessage> Get(string path);
    }

}
