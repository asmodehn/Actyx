﻿using System;
using System.Threading.Tasks;
using System.Collections.Generic;

namespace Actyx
{
    class Program
    {
        static async Task Main(string[] args)
        {
            Console.WriteLine("Hello World!");

	    var s = new EventService();

	    var offsets = await s.offsets();
	    
	    Console.WriteLine(string.Join(Environment.NewLine, offsets));

	    return;
	    
	    string query = "'semantics:edge.ax.sf.UiSession'";

	    await foreach (var q in s.subscribeUntilTimeTravel("foo", query, SnapshotCompression.None)) {
	    // await foreach (var q in new EventService().subscribe()) {
	    	Console.WriteLine("ffffff");
		Console.WriteLine(q.Type);

		if (q is Event) {
		    Console.WriteLine((q as Event).Payload);
		    Console.WriteLine((q as Event).CaughtUp);
		}
	    }
        }
    }
}
