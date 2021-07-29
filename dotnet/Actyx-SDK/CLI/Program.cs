﻿using System;
using System.Collections.Generic;
using System.CommandLine;
using System.CommandLine.Invocation;
using System.CommandLine.Parsing;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx.Sdk.AxWebsocketClient;
using Actyx.Sdk.Formats;

namespace Actyx.CLI
{
    class Program
    {
        private static async Task<IEventStore> MkStore(AppManifest manifest, bool websocket, string node)
        {
            var opts = new ActyxOpts()
            {
                Transport = websocket ? Transport.WebSocket : Transport.Http,
            };
            if (!string.IsNullOrWhiteSpace(node))
            {
                var hostPort = node.Split(":");
                switch (hostPort.Length)
                {
                    case 1:
                        opts.ActyxHost = hostPort[0];
                        break;
                    case 2:
                        opts.ActyxHost = hostPort[0];
                        opts.ActyxPort = Convert.ToUInt32(hostPort[1]);
                        break;
                }
            }
            return await EventStore.Create(manifest, opts);
        }

        private static OffsetMap ParseBounds(ArgumentResult res) =>
            Proto<OffsetMap>.Deserialize(res.Tokens[0].Value);

        private static AppManifest ParseManifest(ArgumentResult res)
        {
            if (res.Tokens.Count == 0)
            {
                return new()
                {
                    AppId = "com.example.actyx-cli",
                    DisplayName = "Actyx .NET CLI",
                    Version = typeof(Program).Assembly.GetName().Version.ToString(),
                };
            }
            return Proto<AppManifest>.Deserialize(res.Tokens[0].Value);
        }

        private static Command Query()
        {
            var cmd = new Command("query"){
                new Option<EventsOrder>("--order"){ IsRequired = true },
                new Option<OffsetMap>("--lower-bound", ParseBounds),
                new Option<OffsetMap>("--upper-bound", ParseBounds),
                new Argument<string>("node"),
                new Argument<Aql>("query", res => new Aql(res.Tokens[0].Value)){ Arity = ArgumentArity.ExactlyOne },
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string, OffsetMap, OffsetMap, Aql, EventsOrder>(async (manifest, websocket, node, lowerBound, upperBound, query, order) =>
            {
                var eventStore = await MkStore(manifest, websocket, node);
                await eventStore
                    .Query(lowerBound, upperBound, query, order)
                    .ForEachAsync(e => Console.WriteLine(Proto<IEventOnWire>.Serialize(e, false)));
            });
            return cmd;
        }

        private static Command Subscribe()
        {
            var cmd = new Command("subscribe"){
                new Option<OffsetMap>("--lower-bound", ParseBounds),
                new Argument<string>("node"),
                new Argument<Aql>("query", res => new Aql(res.Tokens[0].Value)){ Arity = ArgumentArity.ExactlyOne },
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string, OffsetMap, Aql>(async (manifest, websocket, node, lowerBound, query) =>
            {
                var eventStore = await MkStore(manifest, websocket, node);
                await eventStore
                    .Subscribe(lowerBound, query)
                    .ForEachAsync(x => Console.WriteLine(Proto<IEventOnWire>.Serialize(x, false)));
            });
            return cmd;
        }


        private static Command SubscribeMonotonic()
        {
            var cmd = new Command("subscribe_monotonic"){
                new Option<string>("--session"){ IsRequired = true, Arity = ArgumentArity.ExactlyOne },
                new Option<OffsetMap>("--lower-bound", ParseBounds){ IsRequired = true, Arity = ArgumentArity.ExactlyOne },
                new Argument<string>("node"),
                new Argument<Aql>("query", res => new Aql(res.Tokens[0].Value)){ Arity = ArgumentArity.ExactlyOne },
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string, OffsetMap, string, Aql>(async (manifest, websocket, node, lowerBound, session, query) =>
            {
                var eventStore = await MkStore(manifest, websocket, node);
                await eventStore
                    .SubscribeMonotonic(session, lowerBound, query)
                    .ForEachAsync(x => Console.WriteLine(Proto<ISubscribeMonotonicResponse>.Serialize(x, false)));
            });
            return cmd;
        }

        private static Command Offsets()
        {
            var cmd = new Command("offsets")
            {
                new Argument<string>("node"),
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string>(async (manifest, websocket, node) =>
            {
                var eventStore = await MkStore(manifest, websocket, node);
                var offsets = await eventStore.Offsets();
                Console.WriteLine(Proto<OffsetsResponse>.Serialize(offsets));
            });
            return cmd;
        }

        private static Command Publish()
        {
            var cmd = new Command("publish"){
                new Argument<string>("node"),
                new Argument<IEnumerable<EventDraft>>("events", (ArgumentResult res) =>
                    res.Tokens.Select(t => Proto<EventDraft>.Deserialize(t.Value)).ToArray()
                ),
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string, IEnumerable<EventDraft>>(async (manifest, websocket, node, events) =>
            {
                var eventStore = await MkStore(manifest, websocket, node);
                var response = await eventStore.Publish(events.Cast<IEventDraft>());
                Console.WriteLine(Proto<PublishResponse>.Serialize(response));
            });
            return cmd;
        }

        static async Task<int> Main(string[] args)
        {
            var events = new Command("events"){
                Offsets(),
                Query(),
                Subscribe(),
                SubscribeMonotonic(),
                Publish(),
            };
            events.AddGlobalOption(new Option<bool>(new string[] { "--websocket", "-ws" }));
            events.AddGlobalOption(new Option<AppManifest>(new string[] { "--manifest", "-m" }, ParseManifest, isDefault: true) { Arity = ArgumentArity.ZeroOrOne });
            var rootCmd = new RootCommand() { events };
            return await rootCmd.InvokeAsync(args);
        }
    }
}