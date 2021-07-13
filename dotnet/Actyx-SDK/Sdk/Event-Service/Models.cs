﻿using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;

namespace Actyx
{
    public class RangeQuery
    {
        /** Statement to select specific events. Defaults to `allEvents`. */
        public IEventSelection Query { get; set; }

        /**
         * Starting point (exclusive) for the query. Everything up-to-and-including `lowerBound` will be omitted from the result. Defaults empty record.
         *
         * Events from sources not included in the `lowerBound` will be delivered from start, IF they are included in `upperBound`.
         * Events from sources missing from both `lowerBound` and `upperBound` will not be delivered at all.
         */
        public OffsetMap LowerBound { get; set; }

        /**
         * Ending point (inclusive) for the query. Everything covered by `upperBound` (inclusive) will be part of the result.
         *
         * If a source is not included in `upperBound`, its events will not be included in the result.
         **/
        public OffsetMap UpperBound { get; set; }

        /** Desired order of delivery. Defaults to 'Asc' */
        public EventsOrder Order { get; set; }
    }

    public class AutoCappedQuery
    {
        /** Statement to select specific events. Defaults to `allEvents`. */
        public IEventSelection Query { get; set; }

        /**
         * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
         * Defaults to empty map, which means no lower bound at all.
         * Sources not listed in the `lowerBound` will be delivered in full.
         */
        public OffsetMap LowerBound { get; set; }

        /** Desired order of delivery. Defaults to 'Asc' */
        public EventsOrder Order { get; set; }
    }

    public class EventSubscription
    {
        /**
         * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
         * Defaults to empty map, which means no lower bound at all.
         * Sources not listed in the `lowerBound` will be delivered in full.
         */
        public OffsetMap LowerBound { get; set; }

        /** Statement to select specific events. Defaults to `allEvents`. */
        public IEventSelection Query { get; set; }
    }

    public class Aql : IEventSelection
    {

        private readonly string aql;

        public Aql(string aql)
        {
            this.aql = aql;
        }

        public string ToAql()
        {
            return aql;
        }
    }

    public interface IEventFns
    {
        public Task<OffsetMap> Present();

        public Task<OffsetsResponse> Offsets();

        /**
         * Get all known events between the given offsets, in one array.
         *
         * @param query       - `RangeQuery` object specifying the desired set of events.
         *
         * @returns A Promise that resolves to the complete set of queries events.
         */
        public Task<IList<ActyxEvent>> QueryKnownRange(RangeQuery query);

        /**
         * Get all known events between the given offsets, in chunks.
         * This is helpful if the result set is too large to fit into memory all at once.
         * The returned `Promise` resolves after all chunks have been delivered.
         *
         * @param query       - `RangeQuery` object specifying the desired set of events.
         * @param chunkSize   - Maximum size of chunks. Chunks may be smaller than this.
         * @param onChunk     - Callback that will be invoked with every chunk, in sequence.
         *
         * @returns A Promise that resolves when all chunks have been delivered to the callback.
         */
        public IObservable<EventChunk> QueryKnownRangeChunked(RangeQuery query, int chunkSize);

        /**
         * Query all known events that occurred after the given `lowerBound`.
         *
         * @param query  - `OpenEndedQuery` object specifying the desired set of events.
         *
         * @returns An `EventChunk` with the result and its bounds.
         *          The contained `upperBound` can be passed as `lowerBound` to a subsequent call of this function to achieve exactly-once delivery of all events.
         */
        public Task<EventChunk> QueryAllKnown(AutoCappedQuery query);

        /**
         * Query all known events that occurred after the given `lowerBound`, in chunks.
         * This is useful if the complete result set is potentially too large to fit into memory at once.
         *
         * @param query       - `OpenEndedQuery` object specifying the desired set of events.
         * @param chunkSize   - Maximum size of chunks. Chunks may be smaller than this.
         * @param onChunk     - Callback that will be invoked for each chunk, in sequence. Second argument is an offset map covering all events passed as first arg.
         *
         * @returns A `Promise` that resolves to updated offset-map after all chunks have been delivered.
         */
        public IObservable<EventChunk> QueryAllKnownChunked(AutoCappedQuery query, int chunkSize);

        /**
         * Subscribe to all events fitting the `query` after `lowerBound`.
         * They will be delivered in chunks of configurable size.
         * Each chunk is internally sorted in ascending `eventId` order.
         * The subscription goes on forever, until manually cancelled.
         *
         * @param query       - `EventSubscription` object specifying the desired set of events.
         * @param chunkConfig - How event chunks should be built.
         * @param onChunk     - Callback that will be invoked for each chunk, in sequence. Second argument is the updated offset map.
         *
         * @returns A function that can be called in order to cancel the subscription.
         */
        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub);
        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub, ChunkingOptions chunkConfig);


        /**
         * Subscribe to all events fitting the `query` after `lowerBound`.
         *
         * The subscription goes on forever, until manually cancelled.
         *
         * @param query       - `EventSubscription` object specifying the desired set of events.
         * @param onEvent     - Callback that will be invoked for each event, in sequence.
         *
         * @returns A function that can be called in order to cancel the subscription.
         */
        public IObservable<ActyxEvent> Subscribe(EventSubscription sub);
    }
}