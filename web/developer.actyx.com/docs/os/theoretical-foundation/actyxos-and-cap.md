---
title: ActyxOS and CAP
---

A discussion of ActyxOS in light of the CAP theorem

#### What are distributed systems and network partitions?

Systems that span more than one node (computer, tablet, etc.) connected over the network are called _distributed systems_; more formally, a system is called distributed if parts of it can fail while others continue to operate, i.e. they exhibit so-called _partial failure_. Such systems suffer from a host of problems unknown by systems that run on a single machine. A nice introduction to this topic can be found in [fallacies of distributed computing](https://en.wikipedia.org/wiki/Fallacies_of_distributed_computing#The_fallacies). The first false claim is that _the network is reliable_. And this is very false indeed; computer networks are not very reliable. Therefore every device connected over a network can experience temporary disconnects, even on an ideally otherwise functioning network. _Network Partition_ is the terminology we use to describe the state of one or more temporarily disconnected nodes in a cohesive distributed system.

_Network Partition_ results in a situation where interactions of a node with the surroundings (e.g. input from the user or output to the user) become disconnected from the interactions of the remaining part of the system. Thus, not only does this node becomes oblivious of what is going on in the rest of the system but the rest of the system is oblivious to the state of this particular node. Now, this is critical, because the user of the partitioned node would make progress unaware of the situation in the remaining part of the system.

We have two possible procedures now, either we allow the user to progress during the partition, thus making the whole system _available_ or make the system _consistent_ (ie. not allowing the operator to progress while they don't have the full view of what is happening in the whole system).

#### Close up on consistency

What exactly is _consistency_? Let's make an example. Imagine a banking system that keeps the track of account balances. Imagine having a constraint that a customer is not allowed to have a negative balance (ie. owe money to the bank). Let us call the customer Alice. So Alice lives in London and her bank balance is 1000 dollars. Now the system gets partitioned and the New York node does not see the rest of the system. Alice withdraws 500 dollars from the account in London, then boards a fast plane and off she goes shopping in New York! When she arrives there, the system is still partitioned and Alice's balance on New York node is still 1000 dollars. Yay! She buys some stuff that costs her 900 dollars and pays with a card. The transaction goes through because, from the perspective of the New York system, this leaves her with 100 dollars still in the account. Then suddenly, the network connectivity is restored and Alice ends up with her account state being -400 dollars, which violates the _no negative balance_ rule.

In allowing the transaction to proceed on New York's partitioned node, we violated the _consistency_ condition that says '_every read receives the most recent write or an error'._ The New York node has not seen the _most recent write_ that has happened in London and thus the global constraint could not be upheld. Instead, the system chose _availability_, which is _every request receives a (non-error) response – without the guarantee that it contains the most recent write_.

Consistency allows the builders of systems to maintain the illusion of a "system as a whole". So all parts of the system have exactly the same information about its global state, thus allowing it to uphold global constraints. Consistency is a natural descendant of centralized systems where all processing was being done by a single node. It also makes writing programs easier.

But is consistency essential? We would say the answer is "no". Some problems can be naturally re-stated without the requirement of consistency. Factory shop floor processes are naturally localized, they happen at workstations or islands that are closely knit together. Traditionally banking has been done by the means of branches (the SWIFT transfer code still expects a branch designator, most banks nowadays give just one "global" branch, but all this is relatively recent). So in the example above Alice would go to her London branch and ask them to make 500 dollars available to her at the New York branch. She would then have (temporarily) her balance in London reduced to 500 and her balance in New York to be the other 500 (the total balance would still read 1000). Then the system can become partitioned but Alice will not be able to overspend in New York. The same principle is still employed for ATM operations, as the machines can be partitioned pretty frequently. If the machine cannot contact the bank, it can still choose to offer a withdrawal of a small amount of cash. The same goes for offline card transactions.

#### The CAP theorem

Now we are finally ready to discuss the so called [CAP theorem](https://en.wikipedia.org/wiki/CAP_theorem), also named **Brewer's theorem** after computer scientist [Eric Brewer](https://en.wikipedia.org/wiki/Eric_Brewer_(scientist)), who first stated it as a loose conjecture — it was proven in 2002 by Seth Gilbert and Nancy Lynch. This theorem states that it is impossible for a distributed application to simultaneously provide more than two out of the following three guarantees:

-   Consistency: Every read receives the most recent write or an error
-   Availability: Every request receives a (non-error) response – without the guarantee that it contains the most recent write
-   Partition tolerance: The system continues to operate despite an arbitrary number of messages being dropped (or delayed) by the network between nodes

No distributed system can eschew partition tolerance (only centralized systems do, and this is why then can be CA: consistent and available at the same time). For a distributed system the choice is between CP (consistent and partition tolerant) and AP (available and partition tolerant).

The CAP theorem is also very intuitive. When you think about the very definition of consistency, if a system wants to stay consistent, it cannot make any progress (i.e. be available to the users) during the partition, because there might be some writes to some other node that the given node does not know about. Conversely, if it makes progress during partition (is available), then it will not be consistent, because the read that was used to make progress might not be based on the most recent write.

#### The world is not black and white

The CAP theorem speaks only about systems that are consistent in a global manner. For the system to be considered consistent, every read needs to observe _all_ writes, even the ones it does not depend on. Imagine a system that processes users' accounts, and the users are partitioned into nodes by the first letter of their surname. This system could still give consistent outcomes (i.e. the same outcomes as a consistent system would), despite read not observing writes globally, because to work correctly a read for a particular person's account needs only to observe the writes for the same account, occurring on the respective node. Thus no network partition will render the system inconsistent, though the system will also not be fully available for certain operators (the ones on the other side of the partition).

We can continue further, and research is currently ongoing. An example of a further topic would be to allow reads to declare what writes they causally depend on, and if they would be able to see all the writes they need they can still make progress, despite the system not being consistent in the sense of the strong definition of the CAP theorem. One of the examples is the [CALM approach](https://blog.acolyer.org/2019/03/06/keeping-calm-when-distributed-consistency-is-easy/).