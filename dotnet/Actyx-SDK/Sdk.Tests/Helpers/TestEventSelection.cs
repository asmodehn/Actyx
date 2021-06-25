﻿using Actyx;

namespace Sdk.Tests.Helpers
{
    class TestEventSelection : IEventSelection
    {
        private readonly string query;

        public TestEventSelection(string query)
        {
            this.query = query;
        }

        public string ToAql()
        {
            return query;
        }
    }
}
