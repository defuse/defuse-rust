---
title: "Mitigating 51% Attack Risk on the Zcash Network"
date: 2023-09-21T01:00:00-07:00
author: "Taylor Hornby"
draft: false
---

Recently, [Coinbase
observed](https://www.coinbase.com/blog/security-psa-observed-risks-in-zcash-mining-pool-distribution)
that a mining pool called ViaBTC currently controls over 50% of Zcash's hashrate, putting
the network at risk of a "51% attack".

## tl;dr

- So far, no 51% attack has been observed on the Zcash network.
- If you're a ZEC miner using the ViaBTC pool, switching to a different pool
will help prevent a possible 51% attack, protecting the
value of your mining rewards. 
- If you're a Zcash user and want to protect yourself from any future 51%
attack, wait for more than 100 confirmations.
- Zcash nodes will halt if they see a rollback of 100 blocks or more, giving
developers time to investigate and respond to any 51% attack that attempts to
roll back that many blocks.
- Electric Coin Company is working on transitioning Zcash from proof of work to
proof of stake, which should help decentralize the network's consensus,
lessening the risk of 51% attacks in the future.

## What is a 51% attack?

In a proof-of-work blockchain like Zcash, the network reaches a consensus on the
inclusion and ordering of transactions through a process of mining "blocks".

Miners assemble outstanding transactions into a block and then compete to be
the first to solve a crypto-puzzle related to their proposed block. The first
miner to find a solution gets rewarded for their efforts: their block becomes
the next block in the block chain, and they receive a portion of the new funds
created by their block. Miners solve these puzzles by brute-force guessing, and
the rate at which they can try guesses is known as the "hashrate."

As miners carry out this process, there can be competing "branches" of the block
chain. For example, if two miners find a solution around the same time, it's
ambiguous which of their blocks is the actual next block. When this happens,
other miners decide which branch to continue mining on, usually choosing the
first branch they receive so that they can continue mining as quickly as
possible.

Nodes of the network decide which branch of the chain to follow by adding up the
total amount of work it took to solve all of the crypto-puzzles along the
branch. Eventually, any ambiguity about which branch to follow gets resolved
when one branch clearly has the most work applied to it.

This makes it hard to reverse transactions that have been mined to a sufficient
depth. For example, suppose a transaction has been mined into a block, and 10
more blocks have been mined after that one. For an attacker to roll back the
transaction, they would have to out-compete all of the other miners, producing
an alternative chain, which doesn't include the transaction they want to roll back,
and that has more total work applied to it than was required to produce the 10
blocks on the original chain.  So, as long as no one controls a majority of the
network's hashrate, transactions cannot be rolled back.

When a single miner or colluding group of miners controls more than 50% of the
hashrate, they have the power to roll back transactions. This is because they
*can* out-compete all of the other miners and produce an alternate chain that
has more total work. 

Rolling back transactions could be used for theft. For example, the attacker
could make a transaction that sends tokens to an exchange, sell those tokens for
cash, and then roll back the transaction so that the exchange does not receive
the tokens. The attacker leaves with both their original tokens and the cash they
"sold" them for.

As Coinbase pointed out, ViaBTC, controlling more than 50% of Zcash's hashrate,
is in a position where they could carry out such an attack.

## What defenses does Zcash have against 51% attacks?

Zcash nodes each implement [a
rule](https://github.com/zcash/zcash/blob/c908d4bb9ae3be92da14c81ff7a6ddec904c9226/src/main.cpp#L6045-L6061)
whereby the node will shut down if a new chain with more total work appears that
would roll back 100 blocks or more.

This means that, while a 51% attack that rolled back 99 blocks or fewer would
not automatically be detected, a 51% attack that attempted to roll back more
blocks would either not propagate through the network or would cause the network
to halt.

The halting behavior ensures that any large 51% attack will be detected and can
be responded-to by the engineers maintaining Zcash. This means that a user can
reduce their risk of losing funds through a 51% attack by waiting for at least
100 confirmations. If they are then 51%-attacked, the alternate chain is not
likely to become the main chain, and a manual response and resolution by the
engineers maintaining Zcash is likely.

## What can we do to reduce the risk of 51% attacks?

As mentioned above, Zcash users can reduce their risk of losing funds due to a
51% attack by waiting for 100 or more confirmations. Advanced users can also
detect large rollbacks of fewer than 100 blocks, after they happen, by examining
the output of the `getchaintips` command on their node.

Zcash miners using ViaBTC can immediately reduce the risk of a 51% attack by
switching pools. Doing so will reduce the amount of hashrate controlled by
ViaBTC and increase the hashrate controlled by other pools, reducing ViaBTC's
ability to carry out an attack. Miners have incentive to do this, since
protecting Zcash against 51% attacks is also protecting the value of their
mining rewards.

It's also worth noting that ViaBTC is incentivized to *not* 51%-attack the Zcash
network, since if they do, it will likely be discovered, and they risk
tarnishing their brand and losing miners for all of the other blockchains that
they support. That said, Zcash's security should be grounded more soundly than
relying on any one organization's incentives.

As a more thorough solution, Electric Coin Company is [researching and
developing](https://electriccoin.co/blog/the-trailing-finality-layer-a-stepping-stone-to-proof-of-stake-in-zcash/)
a transition to proof of stake for the Zcash network.

Proof of stake is an alternative to proof of work where consensus is no longer
determined by the speed of solving crypto-puzzles, but by validators, selected
according to the amount of funds that have been entrusted to them ("staked").
Misbehaving validators have their funds "slashed", so users are incentivized to
delegate their funds only to trustworthy validators.

Proof of stake consensus algorithms can also suffer from centralization risks.
Attacks similar to 51% attacks are possible against a proof of stake blockchain
when a single entity controls a majority of the network's 
validators. Even so, proof of stake should be an improvement over proof of
work, since it opens the door to more users participating in the network's
consensus.  

To participate in consensus on a proof of work blockchain, you need to own or
rent expensive mining hardware. Not everyone can do this. With proof of stake,
on the other hand, any ZEC holder will be able to delegate their tokens to a
validator to stake. With the barrier to participating in consensus lowered, more
people will be able to participate, which should increase decentralization and
improve Zcash's security against such attacks.

## Conclusion

To summarize, the mining pool ViaBTC currently controls more than 50% of Zcash's
hashrate, enabling them to 51%-attack the network if they choose to do so. So
far, no attack has been observed. 

Users can protect themselves from thefts by waiting for more than 100
confirmations, and Zcash miners using ViaBTC can protect the value of their
mining rewards by switching pools. 

Electric Coin Company is working on transitioning Zcash to proof of stake, which
should allow for easier participation in the network's consensus, thereby
increasing Zcash's decentralization and security.