﻿using Actyx.Sdk.Utils.Extensions;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.AxWebsocketClient
{
    public static class Proto<T>
    {
        public static T Deserialize(string json)
        {
            var settings = HttpContentExtensions.JsonSettings;
            return JsonConvert.DeserializeObject<T>(json, settings);
        }
        public static string Serialize(T value, bool pretty = true)
        {
            var settings = HttpContentExtensions.JsonSettings;
            settings.Formatting = pretty ? Formatting.Indented : Formatting.None;
            return JsonConvert.SerializeObject(value, settings);
        }
    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface IErrorKind
    {
        public string Type { get; }
    }

    public class UnknownEndpoint : IErrorKind
    {
        public string Type { get; } = "unknownEndpoint";
        public string Endpoint { get; set; }
        public string[] ValidEndpoints { get; set; }
    }

    public class InternalError : IErrorKind
    {
        public string Type { get; } = "internalError";
    }

    public class BadRequest : IErrorKind
    {
        public string Type { get; } = "badRequest";
        public string Message { get; set; }
    }

    public class ServiceError : IErrorKind
    {
        public string Type { get; } = "serviceError";
        public JToken Value { get; set; }
    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface IRequestMessage
    {
        public string Type { get; }
        public long RequestId { get; }
    }

    public class Request : IRequestMessage
    {
        public string Type { get; } = "request";
        public string ServiceId { get; set; }
        public long RequestId { get; set; }
        [JsonProperty(NullValueHandling = NullValueHandling.Include)]
        public JToken Payload { get; set; }
    }
    public class Cancel : IRequestMessage
    {
        public string Type { get; } = "cancel";
        public long RequestId { get; set; }
    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface IResponseMessage
    {
        public string Type { get; }
        public long RequestId { get; }
    }
    public class Next : IResponseMessage
    {
        public string Type { get; } = "next";
        public long RequestId { get; set; }
        public JToken[] Payload { get; set; }
    }
    public class Complete : IResponseMessage
    {
        public string Type { get; } = "complete";
        public long RequestId { get; set; }
    }
    public class Error : IResponseMessage
    {
        public string Type { get; } = "error";
        public long RequestId { get; set; }
        public IErrorKind Kind { get; set; }
    }
}
