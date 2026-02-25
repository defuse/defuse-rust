---
title: "Making Zcash Light Wallets Faster and More Private"
date: 2023-03-02T01:00:00-07:00
author: "Taylor Hornby"
draft: false
katex: true
---

In this post, I'm going to sketch changes to the Zcash protocol that would allow
light wallets to be both *much faster* and *more private* than they currently
are. By writing this post, I hope to start a discussion about how we can
pragmatically fix the privacy leaks in the current light wallet protocol and at
the same time vastly improve light wallet performance.

***Warning:** There are almost certainly security bugs in the protocol described below that I haven't caught,
so please don't implement this without careful thought!*

## Two Kinds of Wallet

The proposed design would allow users to make a choice between two kinds of
wallet depending on their preferences for privacy versus performance.

### Option 1: Privacy-Optimized Wallet

A *privacy-optimized wallet* uses trial decryption to find its notes, just like a
full node does. The wallet is careful to always download every transaction,
including all of the data it might ever need such as memo fields. As a result,
the externally-visible behavior of the wallet reveals no information about how
many transactions it is receiving. The cost, of course, is that trial-decrypting
every transaction in the blockchain is bandwidth- and compute-intensive, so
this option is best suited to wallets running on PCs and servers.

This option is for users who need extra-strong privacy guarantees, including
*address unlinkability privacy*, where even an adversary who knows a wallet's
address and has compromised the light wallet server cannot find out which wallet
the address belongs to. In the performance-optimized option, we will give up
this privacy property; this is likely necessary, as we will discuss later.

### Option 2: Performance-Optimized Wallet

In the proposed design, a *performance-optimized wallet* relies on a set of
partially-trusted *Mix Authority* servers as well as a partially-trusted
*Detection Server* to quickly detect their transactions. What these servers do
is explained later on.

A performance-optimized wallet has the following performance and privacy
properties:

#### Performance Properties

- The wallet must authenticate and sync the note commitment and nullifier sets.
- When the wallet is launched, it has to download and trial-decrypt *at most
the current day's worth of transactions*. 
- For all days previous to the current one, the wallet can fetch its
transactions quickly from its Detection Server in a way that *does not require
the Detection Server to scan over all transactions for each user*, i.e. a single
Detection Server could reliably serve thousands or more users.
- If the wallet's chosen Detection Server goes down, or the user switches
Detection Servers, the new Detection Server will have to perform trial-detection
(a linear scan over all new transactions) on the user's behalf until the user
updates their addresses.
- At least one of the Mix Authorities must be operational, otherwise wallets
must find their transactions by downloading everything and using
trial-decryption.
- The wallet must still update the witnesses for their unspent notes as usual.
- The sizes of addresses and transactions are moderately increased.

#### Privacy Properties

- When implemented carefully, even global passive adversaries observing
encrypted network traffic learn nothing about when or how many transactions the
wallet receives, i.e. the traffic-analysis side-channels on the receiving side
are eliminated.
- When a malicious or compromised Detection Server knows one of its users'
wallet's addresses, it can find out which user it is (i.e. identified by their IP address).
- If the Detection Server is compromised but the Mix Authority servers are
honest and uncompromised, the attacker can learn *how many* transactions the
wallet receives each day, but not *which ones*. 
- If a Mix Authority is compromised but the wallet's Detection Server remains
honest and secure, the attacker learns nothing about which transactions belong
to which wallets.
- Only if *both* a Mix Authority *and* the wallet's Detection Server are
compromised, the attacker learns exactly which transactions belong to the
wallet.

When addresses are kept secret, the design also lets wallets granularly delegate
detection authority, i.e. only grant detection authority for certain days, which
further limits the effects of any server compromise that is detected and
corrected. Notably, when addresses are kept secret, even if the private state of
all Mix Authorities and Detection Servers leaks, the attacker can group
transactions within a day by wallet, but they still cannot tie those groups to
wallets' identities[^1] or connect the groups from different days.

[^1]: Assuming the Detection Server never logs users' IP addresses along with
the tags users look up, and that the Detection Server's database retains no
trace of the order in which tags are looked up.

Although the performance-optimized protocol intentionally trades-off privacy and
adds a dependency on "1-of-2" trusted servers for the sake of performance, its
privacy properties are still much better than the reality today where the light
wallet server can learn precisely which transactions belong to which wallets,
confirm address ownership, and more[^2].

The partially-trusted Mix Authorities can eventually be replaced by a
multi-party computation protocol or by a proper mixnet, eliminating the need to
use centralized services and increasing security (e.g. by requiring any
attackers to compromise multiple Mix Authorities rather than just one). One
possibility is to get proof-of-stake validators to do the mixing in a future
proof-of-stake protocol.

[^2]: See the [Zcash Wallet App Threat
Model](https://zcash.readthedocs.io/en/latest/rtd_pages/wallet_threat_model.html)
for details about the current light wallet privacy weaknesses.

## Protocol

In the suggested protocol, Zcash blocks are divided into daily "epochs."
Performance-optimized wallets retain their privacy for the current epoch by
downloading and trial-decrypting all transactions within the current epoch. To
detect transactions in previous epochs, the protocol is augmented as follows.

The Zcash consensus rule maintainers (ECC and ZF) take community input to
maintain a permissioned list of Mix Authorities and the Zcash consensus rules
publish a set of Mix Authority public keys \\(\\{M_{i}\\} \\) for the Mix
Authorities that are active in the current epoch.

### Address Generation

To construct an address, the wallet first generates all of the usual components
of an address as normal, then derives secrets \\(s, r, dsk\\) from their viewing
key.

Then, the wallet obtains a verifiably-randomized public key from their Detection Server:

1. The Detection Server publishes their public key \\(S = [ssk]G_1 \\) for all
users to see. Users check that they all see the same \\(S\\), e.g. using a
public forum or by having it hard-coded into their wallets.
2. The wallet sends \\(r\\) to the Detection Server.
3. The Detection Server computes \\(G_r = GroupHash(r)\\) and \\(S_r =
[ssk]G_r\\) and sends these values to the wallet, along with a zero-knowledge
proof that \\(S_r\\) was computed correctly.
4. The wallet verifies the zero-knowledge proof.

This allows the Detection Server to use its \\(ssk\\) to efficiently decrypt
messages sent to any \\(S_r\\) generated through this process without letting
the Detection Server determine *which* particular \\(S_r\\) the message was
encrypted to. (This is similar to how diversified addresses work, except we
don't want the Detection Server to be able to know which "diversifier" was
used.) This is done to ensure that we never give the Detection Server unlimited
detection capabilities as long as the address is kept secret from them. Checking
the consistency of \\(S\\) between users ensures that the Detection Server is
not using unique private keys in order to identify users.

Next, the wallet generates two extra elements for their address:

1. A transaction tagging key \\(K_{TT} = H(s)\\), where \\(H\\) is a
collision-resistant PRF.
2. A detection public key \\( D = [dsk]G_2 \\).

Given the wallet's normal address \\(addr \\), the wallet's address is now
\\((addr, G_r, S_r, K_{TT}, D)\\).

If the wallet chooses to be privacy-optimized, i.e. to opt-out of efficient
detection, it sets \\(G_r, S_r, K_{TT}, D\\) all to random (but valid for their type)
values.

### Sending Transactions

Given a wallet's address \\((addr, G_r, S_r, K_{TT}, D)\\), a sender constructs a
transaction as follows, in addition to all of the normal steps of constructing a
transaction.

\\(e\\) is the current epoch number, \\(Enc(pk, ptxt, ad, [G = G_3, esk
\\leftarrow \\$, epk = [esk]G])\\) is a DH-based key-private authenticated
public key encryption algorithm with additional data which can be the same as
the one used for Orchard note encryption, and \\(PRF^T, PRF^D\\) are
collision-resistant PRFs.

1. Let \\(C\\) be the usual note ciphertext and construct an additional note
ciphertext \\(C^L\\) that uses a fresh random ephemeral key[^3].
2. Generate an ephemeral keypair \\(esk, epk = [esk]G_r\\).
3. Construct the transaction tag \\(T = PRF^{T}(K_{TT}, G_r\|\|e) \\).
4. Construct a proof \\(\pi\\) that for public inputs \\(T, epk\\), the
transaction creator knows witnesses \\(G_r, esk\\) such that \\(T\\) was
calculated as above and \\(epk = [esk]G_r\\), i.e. \\(epk\\) uses the same base
as was used in the calculation of \\(T\\).
5. Construct the tag ciphertext \\(C^T = Enc(S_r, T\|\|\pi, "tct" \|\| e, G_r, esk, epk)\\).
6. Construct the epoch detection public key \\(D^* = D + [PRF^D(K_{TT}, e)]G_2\\).
7. Construct the detection ciphertext \\(C^D = Enc(D^*, "true", "dct" \|\| e)\\)
8. Construct the post-mix ciphertext \\(C^{PoM} = (C^T, C^D, C^L)\\).
9. Construct the pre-mix ciphertexts \\(C^{PrM}_i = Enc(M_i, C^{PoM}, "mix" \|\| e)\\).

The new note ciphertext is \\( (C, \\{C_i^{PrM}\\}) \\). All of the other
transaction components are constructed as usual.


[^3]: The format of \\(C^L\\) has to be a bit different than \\(C\\) because of
[ZIP 212](https://zips.z.cash/zip-0212); changing rseed would change rcm, which
we don't want, so the defenses in ZIP 212 need to be implemented differently for
\\(C^L\\).

### Mixing and Detection

At the end of each epoch, each Mix Authority \\(i \\) collects the list of
pre-mix ciphertexts \\( \\{ C_{i,j}^{PrM} \\} \\) for that epoch, decrypts them,
and publishes and signs the *sorted and deduplicated* list of post-mix ciphertexts. By
sorting and deduplicating the list, the Mix Authority makes it impossible for
others to tell which pre-mix ciphertexts (outputs on the blockchain)
correspond with which post-mix ciphertexts. In other words, without additional
information, each post-mix ciphertext is hiding in an epoch-sized anonymity set
of shielded outputs.


The deduplication is necessary to prevent attackers from "tagging" ciphertexts
through the mixing process by repeating them a unique number of times. The
\\("mix"\|\|e\\) additional data is necessary to prevent attackers from
replaying honestly-created pre-mix ciphertexts across epochs to circumvent the
mixing. The sorting needs to happen in constant time, otherwise the running time
of the sorting algorithm could leak information about connections between
pre-mix and post-mix ciphertexts.

As long as *at least one* Mix Authority is online, the post-mix ciphertexts will
become available. As long as *all* Mix Authorities are honest and uncompromised,
the mapping between transactions and post-mix ciphertexts will remain secret.
(These properties can be strengthened by onion-encrypting the pre-mix
ciphertexts to follow different paths through multiple levels of Mix
Authorities, as is done in a mixnet.)


Once at least one of the Mix Authorities have published the list of post-mix
ciphertexts, all of the Detection Servers obtain and authenticate the list.
Then, for each \\(C^{PoM}_i = (C^T_i, C^D_i, C^L_i)\\), they use their secret
key \\(ssk\\) to try to decrypt \\(C^T_i\\). If this works, they obtain \\(T\\)
and \\(\pi\\), and after checking \\(\pi\\), they enter \\(C^{L}_i\\) into their
database indexed on \\(T\\).

### Receiving Transactions

All wallets must always download and authenticate the entire note commitment and
nullifier sets. The protocol then allows wallets to detect their incoming
transactions in three different ways. 

#### 1. Trial Decryption

First, the wallet can use its viewing key to trial-decrypt the regular note
ciphertext \\(C \\) as is done currently. This remains possible even if all of
the Mix Authorities and Detection Servers go offline.

#### 2. Indexed Detection

Second, the fastest way for a wallet to obtain its transactions for an epoch
\\(e\\) is to ask its Detection Server for a random nonce \\(n\\) and then
prove in zero-knowledge, for public parameters \\(e, n, T\\), that it knows the
secrets \\(s, r\\) such that \\(T = PRF^T(H(s), GroupHash(r)\|\|e)\\).

The Detection Server authenticates the user by checking the zero-knowledge proof
and the freshness of the nonce and then provides the wallet with all of the
\\(C^L\\) ciphertexts it has indexed on \\(T\\) using a constant-bandwidth protocol.

#### 3. Non-Indexed Detection

If the wallet's Detection Server has gone offline, the wallet has a third option
for efficiently retrieving its transactions from a different Detection Server.
To do this, it constructs its epoch detection secret key \\(dsk + PRF^D(K_{TT},
e)\\) and provides it and the epoch number \\(e\\) to the new Detection Server.
The Detection Server uses this key to trial-decrypt each \\(C^D\\) in the epoch, and
when decryption succeeds, it provides the wallet with the corresponding \\(C^L\\) over
a constant-bandwidth protocol.

When using indexed or non-indexed detection, the wallet obtains and decrypts the
\\(C^L\\) ciphertexts then authenticates their contents by re-computing the note
commitment and checking that the same commitment appears in the authenticated
note commitment set at a position within the expected epoch.

### Security Analysis

#### Address Unlinkability

When a wallet's Detection Server knows the wallet's address, the detection
server can identify which of its users owns the wallet (e.g. by IP address).

This seems unavoidable when the wallet doesn't download everything, because with
the address known, the Detection Server can always send thousands of
transactions to the address. Assuming this doesn't cause *all* wallets to
download thousands of transactions worth of data, the victim wallet then either has
to make itself distinguishable from other wallets by downloading thousands of
transactions worth of data or fail to receive many of its transactions,
suggesting a DoS attack on addresses is possible. Either *all* wallets download
lots of data when
*any* wallet needs to, DoS attacks on addresses are possible, or wallets with
addresses that receive lots of transactions are identifiable by the amount of
data the Detection Server sends to them.

Only the Detection Server observes this leakage, not a global passive adversary,
because the Detection Server always sends detected transactions over a
constant-bandwidth protocol (e.g. in fixed-sized packets sent at regular
intervals).

*Open Question:* Can we prove that this limitation applies to all possible
protocols?

#### Granular Delegation of Detection Authority

When the user's chosen Detection Server knows their wallet's address, it can
perpetually find out which \\(C^L\\) belong to the wallet. This is because,
knowing \\(G_r, K_{TT}\\), it can re-compute the address's tags for each epoch.
This is always possible for any design that allows the Detection Server to
"bucket" transactions for future retrieval by wallets: the Detection Server can
send many transactions to the known address and watch which "bucket" grows. As
long as the Mix Authorities are secure, even a Detection Server that knows a
wallet's address will only learn *how many* transactions the wallet receives
each day.

When the wallet's address is kept secret from the Detection Server, the
Detection Server is unable to reliably detect transactions for the wallet in
epochs other than the ones explicitly requested by the wallet. 

In the indexed detection case, this is because given the epoch-specific tag \\(T
= PRF^T(K_{TT}, G_r\|\|e)\\), the Detection Server cannot compute the tag for
any other epoch without knowing \\(K_{TT}\\). For other epochs, the
Detection Server only learns, "one of my users' wallets received \\(n\\)
transactions" but it does not know which wallet until the wallet reveals its tag
for that epoch. The anonymity set is lower-bounded by wallets using the same
Detection Server whose addresses are also secret and who have not provided their
epoch-specific tags. This is especially useful if a Detection Server is known to
be compromised and all users stop using it. Additionally, any user of the
Detection Server can increase the size of the anonymity set by registering
addresses to which they send random amounts of "dummy" transactions.

Since the Detection Server only ever needs ephemeral access to the tags wallets
request, the Detection Server can be careful to delete all trace of the
relationship between wallets' identities and their requested tags after the
requests have been served. This ensures that if the Detection Server's state
gets leaked, it will be apparent how *mix-anonymized ciphertexts* are grouped
together within epochs, but for addresses that were kept secret, those groupings
(daily transaction counts) will not be tied to wallets' identities in any way.
If Mix Authorities' keys are leaked too, it will be apparent how *transactions*
are grouped daily, but those groups won't be linked to wallets' identities when
addresses are kept secret.

In the non-indexed case, the Detection Server learns \\(dsk + PRF^D(K_{TT},
e)\\). Without knowing \\(K_{TT}\\), they cannot recover \\(dsk\\) or any of the
other per-epoch detection secret keys. This is useful in case the user's main
Detection Server goes down and they are forced to use a less-trustworthy one. As
long as their address is kept secret, the new Detection Server is only granted
detection authority for a limited number of epochs.

Using Identity-Based Encryption (IBE), the non-indexed case can probably be improved
so that it does not rely on address secrecy. I am currently investigating the
impact IBE would have on address sizes.

In all cases, it is impossible to link a pre-mix ciphertext to a post-mix
ciphertext unless either you can decrypt a message encrypted to a Mix
Authority's public key key or you created the transaction yourself. This
guarantees that even if the Detection Server learns a user's address, their true
notes are still hiding in an epoch-sized anonymity set. At most, they learn how
many notes the user receives in each epoch, and as explained earlier, can
identify the address owner (by IP address) when they connect and ask for their
transactions.


#### Obfuscating the number of transactions received

As long as the Mix Authorities are secure, wallets can send random amounts of
transactions to themselves to obscure the total number of transactions that they
receive. The Detection Server is unable to tell which transactions are
self-transactions, so they would see the distribution \\(R + F\\), where \\(R\\)
is the distribution of real transactions the wallet receives and \\(F\\) is the
distribution of fake transactions the wallet generates to itself. This would
require wallets to be online daily to send fake transactions in each epoch, or
they could delegate the task to a third-party service by providing their
address.

#### Lack of post-quantum privacy

The suggested protocol changes are *not* post-quantum private.  Large-scale
quantum computers would be able to reveal the connection between pre-mix and
post-mix ciphertexts. Additionally, quantum computers would be able to decrypt
the tags.  So, even when addresses are kept secret, quantum computers would be
able to group transactions with identical tags within epochs. In other words, a
quantum attacker has the same capabilities as an attacker who had compromised
all Mix Authorities and Detection Servers. This is a *weakening* of Zcash's
post-quantum privacy properties, since currently Zcash is conjectured to be
post-quantum private when addresses are kept secret.

#### The need for \\(\pi\\) in step 4 of transaction generation

The Detection Server checks the proof \\(\pi\\) generated in step 4 and makes
sure that wallets fetching tags know \\(r\\) in order to prevent senders from
being able to learn which Detection Server an address uses. 

Without these checks, an attacker (a) could use a *different* \\(G_{r*}\\) from
the same server as the encryption base while still computing the tag with
\\(G_r\\) and the victim would receive the transaction if and only if the
attacker used the same server, or (b) could send a transaction to themselves (a
\\(K_{TT}\\) for which they know \\(s\\)) using the victim's \\(G_r\\) for both
the encryption and \\(T\\); they would receive the transaction if and only if
they used the same server.

In case (a), \\(\pi\\) ensures that the tag \\(T\\) is always computed using the
same base as was used for the encryption, so if an attacker tried this, the
victim would not receive the transaction because they would never request the
attacker-generated tags that use a different base. In case (b), the attacker is
not able to look up tags constructed using the victim's \\(G_r\\) because the
Detection Server requires them to prove knowledge of \\(r\\) when requesting a
tag.

This kind of attack is possible in general for any protocol by DDoSing a
Detection Server and observing if that prevents the victim from receiving their
transaction. However, using a DDoS attack to carry out the attack would be much
more obvious than exploiting the weaknesses that would exist without these
checks.

With these checks in place, the protocol has better privacy properties for receivers than the
state-of-the-art mixnet design
[Loopix](https://github.com/zcash/zcash/issues/288#issuecomment-658348272),
which requires senders to know which Detection Server ("providers" in their
terminology) the recipient uses.

### Optimizations and Improvements

\\(s\\) and \\(r\\) can be combined by using \\(G_s = GroupHash(s)\\) instead of
\\(G_r = GroupHash(r)\\). However, to do this, the address registration process
must be modified so that the wallet only sends \\(G_s\\) to the server (so that
it doesn't disclose \\(s\\)), and the wallet must prove in zero knowledge that
\\(G_s = GroupHash(s)\\) (otherwise the registration API becomes a
decryption oracle!).

The same protocol can also be used for transaction inputs so that wallets can
detect their spends efficiently.

Identity-based encryption could also potentially be used for Detection Servers'
public keys so that their private keys can be kept offline and only loaded onto
the live server as needed, and old private keys can be deleted after they are no
longer needed. This would provide stronger forward-security and
backward-security properties in case a well-operated Detection Server is
compromised.

Since, assuming address secrecy, indexed lookups across days cannot be linked,
wallets can perform each lookup over a fresh Tor circuit to further obfuscate
the number of transactions their wallet receives over time. This would be vulnerable to
global passive adversaries and to timing correlation attacks, however.

If the additional privacy and compromise-resistance properties for the indexed
detection case that rely on address secrecy are deemed not to be very useful,
the protocol can be greatly simplified using [Daira Hopwood's detection key
design](https://github.com/zcash/zcash/issues/288#issuecomment-658348272).

The Zcash consensus rules could require wallets to pay a fee to the mix
authorities, so that they are sustainably funded.

In the proposed design, wallets only learn about the shielded outputs they
receive, and not other parts of the transaction. In order to learn the other
transaction components, e.g. to be able to compute the txid, the sender would
need to encrypt this additional data to the wallet through the mixing process,
and the wallet would need to authenticate it somehow.

Full nodes can prune all of the pre-mix ciphertexts to save on storage space.

The Mix Authority public keys should be rotated regularly to reduce the impact
of a compromise.

## Sender Privacy

While the above design provides pretty good privacy for transaction receivers,
it leaves open a privacy weakness for transaction senders.

Suppose, for example, that a controversial charity publishes their viewing key
for accounting purposes and that a user wishes to donate to the charity.
Alternatively, suppose that the viewing key was not published but was
compromised by an attacker.

When the user's wallet broadcasts their donation transaction, the attacker can
use the viewing key to try to decrypt the transaction and learn that the user is
donating to the charity.

In general, anyone who has compromised the light wallet server can find out who
is sending transactions to them or to any address whose viewing key they know.

To prevent this, we need to add sender anonymity. This could be done by re-using
the same Mix Authorities to batch and shuffle transactions before they are
broadcast publicly into the blockchain, e.g. using [Loopix](https://www.usenix.org/conference/usenixsecurity17/technical-sessions/presentation/piotrowska)'s Poisson mixing strategy.

Wallets can also use Loopix's concepts of "loop cover" and "drop cover" traffic
to respectively obscure the number of transactions received (as described above)
and eliminate information leakage about how many transactions are sent.

The "drop cover" traffic would never need to be added to the blockchain, and if
the same Mix Authorities are used for sending and receiving, the "loop cover"
traffic for obscuring receive counts need not be entered into the blockchain
either.

## Conclusion; What's Next?

By implementing something like the above protocol, Zcash light wallets could
find their notes much faster while being much more private than they are today.
The drawbacks to this design are that (a) address sizes and transaction sizes
are increased, (b) the protocol relies on a permissioned set of mix authorities,
(c) daily received-transaction-count data resides on Detection Servers and is
vulnerable to being leaked, and (d) the protocol needs three additional
zero-knowledge proofs whose circuits need to be designed and implemented.
Ideally, the permissioned set of mixes would eventually be replaced by a proper
permissionless mixnet, e.g. with the mixing being done by proof-of-stake
validators.

The next steps for me are to,

1. learn about identity-based encryption to see if it's reasonable to eliminate
the address secrecy requirement for granular delegation in the non-indexed detection case,
2. think more carefully about the protocol design to see if it can be simplified
and optimized and to ensure there aren't any vulnerabilities, and
3. collect feedback and write a ZIP.

If you are a cryptographer, please try to break the protocol!