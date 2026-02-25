---
title: "Scalable Private Money Needs Scalable Anonymous Messaging"
date: 2022-11-15T19:00:00-07:00
author: "Taylor Hornby"
draft: false
---

In this post I’m going to argue that any scalable private Internet money system
will need to rely on an equally-scalable and equally-private anonymous messaging
system.

Ultimately, I will argue that the best approach to scaling private money is to
_directly and explicitly_ build a scalable anonymous communication system,
rather than proceeding through a series of ad-hoc improvements to current
designs.

## Backstory

The performance of Zcash mobile wallets has been degraded massively; this was
caused by a huge increase in private transaction load on the network. This
problem is not specific to Zcash—all private money systems aiming to offer
strong privacy guarantees will face this challenge eventually. To understand
why, we need to understand how the current best design for private Internet
money works.

## Protocol Recap

When Alice wants to send Bob a payment, Alice creates a _note_ containing the
value she wants to send to Bob. In her transaction, Alice posts three things to
the public blockchain: a note commitment, a zero-knowlege proof, and a note
ciphertext. 

The note commitment ties the newly-created note to Bob’s secret keys, so that
only Bob can spend the note. The zero-knowledge proof ensures that Alice is
creating the note honestly, i.e. she’s only creating it from money she controls,
and that she isn’t creating new money out of thin air.

The third part, the note ciphertext, contains all of the information Bob
needs—along with his secret keys—to spend the value he has received from Alice.
A key-private encryption algorithm is used to create this ciphertext, which allows
Bob to decrypt it with his secret key, while ensuring nobody—even people who know
Bob’s address—can tell that the ciphertext belongs to Bob.

## The Performance Bottleneck

This design creates a scalability bottleneck: Bob has to try to decrypt every
ciphertext on the blockchain to find the notes that belong to him. When there
are a lot of new notes, his wallet will take a long time to get through them
all. In current implementations, it sometimes takes _days_ of trial-decrypting
to catch up with all of the transactions on the blockchain, depending on how far
behind the wallet is.

This bottleneck isn’t all for nothing, it’s what gives the system its strong
privacy guarantees. 

The information Alice broadcasts in her transaction looks
like random noise, so although an attacker can tell _that_ Alice is sending a
transaction, the attacker can’t tell who it’s going to or what the amount is. On
Bob’s end, an attacker can tell _that_ Bob’s wallet has downloaded _all
transactions in existence_, but the attacker cannot tell _whether or not_, or
_how many_, transactions Bob’s wallet actually received[^1].

[^1]: Note that current implementations of Zcash light wallets don’t take advantage
of these strong privacy properties, because Bob’s wallet will fetch extra
details of just those transactions that belong to him for the sake of bandwidth
savings. See the [Zcash Wallet App Threat
Model](https://zcash.readthedocs.io/en/latest/rtd_pages/wallet_threat_model.html)
for more details.

This everyone-has-to-try-to-decrypt-everything bottleneck is inherent to all
private money designs that offer strong formal privacy guarantees. For these
systems to scale to thousands or millions of active users, trial decryption
needs to be replaced with something else.


## Private Money Implies Anonymous Messaging

To overcome the trial-decryption bottleneck, Bob needs to be able to find out
about his notes—the money he is receiving—in an efficient way. 

We can show formally that solving this problem is closely related to a different
computer science problem: that of building an anonymous messaging system (also
known in the literature as an “anonymous communication network”).

To see why, we can reduce the problem of building an anonymous messaging system
to the problem of building a private money system. That’s computer-science speak
for using the solution to Problem A (building a private money system) to
construct a solution for Problem B (building an anonymous messaging system);
doing so shows that Problem A is at least as hard as Problem B, i.e. to do A you
must do B. To build private money you must build anonymous communication.

Given a private money system, any two users of the system can use it to message
each other privately and anonymously with the same privacy and anonymity
properties as the underlying private money system. They can do this by encoding
their messages into transaction values. If Alice wants to say “Hello” to Bob,
she can encode her message to the ASCII values 72, 101, 108, 108, 111 and send a
series of transactions with values 0.0072, 0.0101, 0.0108, 0.0108, 0.0111, plus
some zero-valued transactions to hide the length of the message.  Simple, right?
Any private money system can be used as an anonymous communication system with
only a constant-factor performance overhead.

What this means is that in order to build a private money system with certain
formal privacy and scalability properties, we are _logically required_ to build
an anonymous messaging system with at least those same privacy and scalability
properties. It also means that research on the trade-offs and limitations of
anonymous messaging systems apply to private money systems as well.

In the other direction, we can _use_ a scalable anonymous messaging system to
overcome the trial-decryption bottleneck of our current private money designs.
Using the anonymous messaging system, Alice can send Bob the ciphertext directly
so that Bob doesn’t have to try to decrypt everything. When we do this, the
privacy and scalability properties of the resulting private money system are
_limited_ by the privacy and scalability properties of the anonymous
communication system.

## What are our options?

Let’s look at a few of the options we have for replacing trial decryption.
There’s _tons_ of research on anonymous messaging systems, so I’m going to
restrict my attention here to options that are already on private-money
projects’ roadmaps or that seem promising to me.

### Smarter Scanning

The option currently being pursued in Zcash is smarter scanning algorithms.
These algorithms must still eventually try to decrypt every ciphertext, but they
can find some of a user’s funds earlier, so that the user has some spendable
money and can make payments faster, without waiting for the entire scan to
complete. It won’t eliminate the long scanning times, but it will make user
experience significantly better.

For more details, see
[https://hackmd.io/@str4d/dagsync-graph-aware-zcash-wallets](https://hackmd.io/@str4d/dagsync-graph-aware-zcash-wallets).

This is a good stopgap measure in the face of bricked wallets, but in the long
term, we will need something more.

### Scanning in the Cloud

Another simple approach is to retain the trial-decryption paradigm, but move it
out of the smartphone and into the cloud, on beefier computers.

This would retain all of the on-chain privacy properties of current protocol
designs, since there are no changes to the data stored on-chain. However, users’
wallets would need to send their decryption keys to the cloud. The user’s
privacy will only be as strong as the security of those cloud computers. (Note
that this can be done without giving _spend authority_ to the cloud servers, so
users' funds are safe.)

If many users share the same cloud computer for scanning, that cloud computer
becomes an attractive target for adversaries who want to de-anonymize a mass of
users. That risk can be reduced somewhat by giving each wallet its own isolated
computer in the cloud, but this increases the cost and setup complexity for
users, and doesn’t isolate users against the mass-exploitation of a common bug
in the cloud setup.

Users’ privacy can be protected further at the cost of increasing CPU load on
the servers with homomorphic-encryption-based [Oblivious Message Retrieval
(OMR)](https://forum.zcashcommunity.com/t/oblivious-message-retrieval/40715).

Cloud scanning is not a good long-term solution because it does not
fundamentally improve the system’s scalability, it just moves it to faster CPUs
(and perhaps later onto cloud GPUs). With enough usage, even scanning in the
cloud will not be fast enough, and will be too costly.

### Fuzzy Message Detection (& Other Decoy Systems)

[Fuzzy Message Detection](https://eprint.iacr.org/2021/089) (FMD) is an
intermediate approach between all-on-the-device scanning and all-in-the-cloud
scanning. 

With FMD, the transaction recipient (or the consensus rules in
[Penumbra’s
case](https://protocol.penumbra.zone/main/crypto/fmd/sender-receiver.html))
selects a false-positive rate and generates a detection key which is sent to the
cloud server. The cloud server uses the detection key to find all of the user’s
transactions, plus a collection of false positives at the chosen rate. The
server cannot tell which transactions legitimately belong to the user or are
false positives, so some uncertainty is created about which transactions belong
to the wallet.

For a false positive rate P ∈ [0, 1], this reduces the amount of transactions
the wallet needs to scan locally by a factor of P. For example, a 1%
false-positive rate would make the wallet’s scanning 100x faster at the privacy
cost of giving the server a list of 99% of all transactions which _definitely do
not_ belong to the wallet.

Unfortunately, we need to be skeptical of FMD’s privacy guarantees. 

The most pressing problem occurs when Alice sends transactions to Bob
repeatedly. An adversary who compromised the FMD server can collect strong
statistical evidence that Alice is paying Bob. For example, suppose that Alice
sends 100 transactions to Bob and the adversary knows all of those transactions
were created by Alice. Even for false-positive rates higher than 50% (less than
2x speedup), if _none_ of those transactions were going to Bob, the chance that
_all_ of them would be downloaded by Bob (as false positives) is less than 1 in
2^100. So, if the adversary sees Bob download them all, they can be nearly
certain Alice is sending payments to Bob, and privacy has been broken.

This particular attack can be mitigated with sender-side anonymity protections,
removing the assumption that the adversary knows Alice created the transactions.
However, other desirable privacy properties remain broken. For example, if the
adversary wants to find out which wallet owns a particular address, they can
send that address 100 transactions and then observe which wallet retrieves all 100
of them.

These attacks are symptoms of a more fundamental problem with FMD, which is that
decoy-based systems are incapable of ensuring formal privacy guarantees.

In [On Privacy Notions in Anonymous
Communication](https://arxiv.org/abs/1812.05638), the authors survey and
systematize desirable formal privacy notions for anonymous messaging. The
authors worked out the logical implications between all of the formal notions in
their systematization. They found that all of them imply a notion called
“sender-receiver pair unlinkability”. That means that if a system doesn’t have
sender-receiver pair unlinkability, it doesn’t have any of the other formal
privacy notions that they considered.

A simplified version of sender-receiver unlinkability is defined by a game. In
the game, a secret bit is chosen at random. If the bit is 0, Sender 1 sends a
message to Receiver 1 and Sender 2 sends a message to Receiver 2. If the secret
bit is 1, the destinations are swapped; Sender 1 sends to Receiver 2 and Sender
2 sends to Receiver 1. To win the game, i.e. to break the privacy property, the
adversary has to guess the secret bit.

With FMD in this four-user scenario, either the false-positive rate is high
enough that both receivers download both senders’ messages (meaning no
efficiency has been gained), or one receiver fails to download one of the
sender’s messages. Since there are no false-negatives in FMD, this means that
the not-downloaded sender’s message was definitely not directed at that
receiver, and the adversary learns the secret bit. For example, if Receiver 1
_didn’t_ download Sender 1’s message, then we know the bit is 1, since if it was
0, Receiver 1 would have definitely downloaded Sender 1’s message.

This shows that FMD cannot satisfy sender-receiver pair unlinkability, and by
the logical implications given in the paper, cannot satisfy any of the other
formal privacy definitions. FMD’s privacy guarantees are necessarily heuristic
and statistical. 

More analysis of FMD’s privacy properties against weaker privacy notions can be
found in [The Effect of False Positives: Why Fuzzy Message Detection Leads to
Fuzzy Privacy Guarantees](https://arxiv.org/abs/2109.06576).

In addition to lackluster privacy properties, FMD is also not scalable. The FMD
server must still process all transactions for each user, and at best it offers
a constant factor reduction in the scanning work that wallets must perform
locally.

### Using Existing Communication Channels

If two users already talk to each other over an existing secure messaging
system, then they can send transactions over the messenger. For those
transactions (but not others), the recipient’s wallet can avoid
trial-decryption. This is the idea behind [liberated
payments](https://github.com/zcash/zips/pull/420).

This scales well, but has two drawbacks.

First, the user experience is a significant departure from what is common among
cryptocurrencies. Users are accustomed to sending money to an address given to
them by their counterparty; with liberated payments, they would have to add
their counterparty as a contact on a messenger and get their wallet to send them
a message. I believe this can be made usable, but the sheer fact it works
differently might be enough to put a lot of users off.

Second, the privacy properties of the resulting system are only as good as the
metadata-resistance privacy properties of the messengers used. For example, if a
transaction is sent through a messenger that uses strong encryption, but the
messenger leaks metadata about message size (identifying the message as likely
being a liberated payment) and who the message is coming from and going to, an
adversary watching this metadata can tell approximately who is sending payments
to who, breaking privacy guarantees that users expect.

Liberated payments would work great for use cases where users are making
payments to a website, for example sending funds to an exchange, buying
something from an online store, or donating to a charity. In this case, any
attackers monitoring the user's Internet connection are already aware that there
is some kind of relationship between the user and the website they have visited.
If the payment information is sent through the same encrypted connection that
the user's browser is already using to communicate with the website, the fact a
payment is being made can be kept secret. If the payment information is sent
through a separate connection (e.g. from the user's smartphone wallet, with the
payment being made by scanning a QR code in a desktop browser), the fact a
payment was made will be leaked, but this might not matter for some use cases,
like if the website is a store and the attacker is already pretty sure that the
user is going to buy something.

So, liberated payments would work well as a workaround to wallet performance
problems for payments to websites and for a subset of users who already have a
private means to communicate with each other, but it may not be a good long-term
solution because of the metadata-privacy weaknesses in existing private
messengers and because it requires users to learn a new UX.

### SGX-based Approaches

Another way to scale would be to use a trusted, private server. Using an actual
trusted server would defeat the purpose of building a private money system (we’d
just be building another PayPal), but Intel’s SGX technology aims to make it
possible to approximate a trusted server in an untrusted way. This is the
approach that [MobileCoin](https://mobilecoin.com/) takes (in combination with
the decoy-based CryptoNote protocol).

Unfortunately, SGX’s security has been broken repeatedly by side-channel attacks
against the processor
([some](https://www.usenix.org/conference/usenixsecurity18/presentation/bulck)
[recent](https://sgaxe.com/)
[examples](https://ieeexplore.ieee.org/abstract/document/9152636)). Like
copy-protection (“DRM”), what SGX is trying to do is fundamentally impossible.
The CPU itself needs access to the private bits in order to compute on them, so
it can _only_ be a matter of _how hard_ it is for an attacker to access those
bits, and how much resources they need to expend to do so. It is conceivable
that one day SGX will attain a state where all known attacks are practically
infeasible, but that’s currently not the case, and we don’t know when (if ever)
that will happen. The *variety* of methods that have recently been used to break
SGX and alternatives like ARM TrustZone is good evidence that there are still
more vulnerabilities waiting to be found, so we should expect researchers to
develop more attacks in the coming years.

In my opinion, the best way to think about SGX’s security is: “If the system’s
operators are mostly honest and aren’t too well-resourced, then you’re safe, but
if the NSA breaks in, or anyone with serious resources wants to break the
system, SGX will fail.” This is not a good long-term solution, at least not
until SGX or an equivalent technology has proven itself to resist all kinds of
attacks for a number of years.

### Tor

Tor is by far the most widely-used anonymous communication system in existence
today. Its focus is on providing low-latency connections at the expense of
formal privacy guarantees.

Tor works by having the client select three “nodes” from the pool of available
nodes: an entry node, a relay node, and an exit node. The client encrypts its
message to the exit node’s key, encrypts that ciphertext to the relay node’s
key, and encrypts that ciphertext to the entry node’s key. The entry node can
decrypt the first layer of encryption and forward it onto the relay node, who
can decrypt one step further and forward it onto the exit node, who decrypts the
last layer and sends the data to the intended destination. The entry node knows
who the user is, the exit node knows where the traffic is going, and the relay
node prevents the entry and exit nodes from finding each other and colluding to
see who’s-sending-what-to-who. Tor also supports onion services, which allow the
service being connected-to to remain anonymous as well.

Tor can be used to build anonymous messaging, a notable example being
[Cwtch](https://cwtch.im/). In this way, it could be used to build an anonymous
messaging system in support of private money.

However, Tor’s design explicitly trades-off worse privacy guarantees in order to
have lower latency. This is necessary for it to be suitable for browsing web
pages. But the low latency makes it vulnerable to timing attacks: after a client
sends a packet, it will shortly thereafter show up at its destination. If an
adversary is situated close to the client user (e.g. at an ISP), and also close
to the destination server (e.g. at an Amazon datacenter), they can collect
enough packet-timing information to statistically infer who’s talking to who. It
is well-known and accepted that Tor’s anonymity can be broken by global passive
adversaries, and adversaries situated at both endpoints, using attacks like
this. A [recent paper](https://ieeexplore.ieee.org/abstract/document/9471821)
surveys different kinds of de-anonymization attacks on Tor.

Tor’s privacy properties are heuristic and are weaker than those that come
standard with projects like Zcash[^2]; they are certainly insufficient for some use
cases of private Internet money. Therefore, extreme care needs to be taken with
any attempt to solve payment scalability problems with Tor. We must be mindful
that relying on Tor will mean giving up on formal privacy guarantees and will
make the system vulnerable to global passive adversaries, as well as adversaries
that are capable of monitoring the traffic of the set of wallets they are
interested in.

[^2]: At least using the full-node implementation.

### Mixnets

As argued by the reduction given above, a private money system fundamentally
_is_ an anonymous messaging system. It therefore makes sense to look towards the
best designs for anonymous communication systems in the literature if we want to
solve private money systems’ scalability problems.

This could be termed the “embrace the real problem” approach. Rather than
working through a series of ad-hoc strategies to scaling private money, all of
which come with their own scalability bottlenecks and privacy weaknesses, it
would be a more effective use of resources to work directly on solving the
_necessary_ problem of scaling anonymous communication.

State-of-the-art designs for scalable anonymous messaging use mixnets. Mixnets
work by collecting users’ messages into batches and using a series of “mixnodes”
to shuffle the messages before delivering them to recipients’ mailboxes. By
shuffling messages in large batches (requiring high latency), and by using
sufficient chaff traffic, mixnets can provide provable privacy guarantees.

Mixnets are not a panacea, however. Fundamental trade-offs apply, such as
between their privacy properties, latency of messages, efficiency, and offline
message availability. They are a newer technology, too, which hasn’t seen much
proven deployment in practice ([Nym](https://nymtech.net/) is one example).

## Conclusion

In this post I’ve surveyed different approaches to solving private money
systems’ scalability problems. I've included those that I consider plausibly
viable as well as those on existing projects' roadmaps. A common theme among
many of them is that they are not asymptotic scaling solutions, only making
constant-factor improvements or moving the processing to faster computers. Most
of them don’t offer formal privacy guarantees or, worse, have known attacks that
make them unsuitable for Internet money. Others, like mixnets, are expensive and
risky to implement.  There is no clear winner.

What _is_ clear, though, is that scaling private payments _is_ scaling anonymous
messaging. 

We should therefore turn our attention toward the best designs available in the
scientific literature on anonymous communication, which in my view is currently
mixnet research.

It’s unlikely that we will find an off-the-shelf solution, ready to be
implemented, that will solve all of our problems.  We will need to make
significant investments in anonymous communication science and engineering over
the long term to make private money work. This is worth doing, because this way,
we are embracing the real, unavoidable, problem that lies ahead of us.

(Some updates to this article were made on 2023-02-16.)