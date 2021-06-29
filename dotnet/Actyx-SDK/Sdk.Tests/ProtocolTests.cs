﻿using System.Collections.Generic;
using Actyx;
using DeepEqual.Syntax;
using Newtonsoft.Json.Linq;
using Xunit;

namespace Sdk.Tests
{
    public class ProtocolTests
    {
        static void Roundtrip<T>(T value)
        {
            var serialized = Proto<T>.Serialize(value);
            var deserialized = Proto<T>.Deserialize(serialized);
            deserialized.ShouldDeepEqual(value);
        }

        [Fact]
        public void Incoming()
        {
            new List<IResponseMessage> {
                new Next { RequestId = 1, Payload = new JToken[] { JToken.Parse(@"{ ""this is"": ""the payload"" }") } },
                new Complete { RequestId = 1 },
                new Error
                {
                    RequestId = 1,
                    Kind = new UnknownEndpoint
                    {
                        Endpoint = "invalid",
                        ValidEndpoints = new string[] { "valid1", "valid2" },
                    }
                },
                new Error { RequestId = 1, Kind = new InternalError { } },
                new Error { RequestId = 1, Kind = new BadRequest { Message = "It's really bad!" } },
                new Error
                {
                    RequestId = 1,
                    Kind = new ServiceError
                    {
                        Value = JToken.Parse(@"{ ""some"": ""nested"", ""props"": ""right here"" }")
                    }
                }
            }.ForEach(Roundtrip);
        }

        [Fact]
        public void Outgoing()
        {
            new List<IRequestMessage> {
                new Request { RequestId = 1, ServiceId = "some_service", Payload = JToken.Parse(@"{ ""this is"": ""the payload"" }")},
                new Cancel { RequestId = 1 },
            }.ForEach(Roundtrip);
        }
    }
}
