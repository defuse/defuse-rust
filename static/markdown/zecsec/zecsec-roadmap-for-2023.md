---
title: "ZecSec Roadmap for 2023"
date: 2023-01-03T11:08:00-07:00
author: "Taylor Hornby"
draft: false
---

Happy New Year Zcash!

For me, the days leading up to the new year are always a time of reflection and
planning. In that spirit, I've laid out a tentative roadmap for the ZecSec
project in 2023.

## Upcoming Audits

### 1. Lightwalletd

All Zcash light wallets rely on the
[lightwalletd](https://github.com/zcash/lightwalletd) project to obtain
transaction information and to broadcast transactions to the network.  Despite
its central importance to the Zcash ecosystem, it has never been subjected to a
third-party audit. I'll begin the year with a thorough security review of its
entire codebase.

### 2. Zondax

Shielded hardware wallets have been a long time coming. Now, [Zondax](https://docs.zondax.ch/Zcash) is ready
with code and integration PRs for shielded Zcash on Ledger hardware. Security review is
important at this early stage to ward off bugs that could put users' funds at
risk, so we'll review their code and integrations next.

### 3. Trezor Shielded

I am also excited to audit [Trezor's support for Zcash shielded
transactions](https://github.com/trezor/trezor-firmware/pull/2472). Having the
two major hardware wallet vendors supporting shielded transactions will be a
major milestone for Zcash!

### 4,5,6. Completing a Round of Wallet Audits

In 2022, I began a series of wallet audits by reviewing [Ywallet](https://ywallet.app/) and
[zecwallet-lite-cli](https://github.com/adityapk00/zecwallet-light-cli), two widely-used wallet codebases which had not received
recent review. Next up, I'll complete the wallet audit series by auditing
[ZecWallet-Lite](https://zecwallet.co/), [Nighthawk](https://nighthawkwallet.com/), and the wallet in development by [ZingoLabs](https://github.com/zingolabs).

### 7. ZSAs

Zcash Shielded Assets (ZSAs) are on the horizon for Zcash, with the [design and implementation being provided by QEDIT](https://forum.zcashcommunity.com/t/a-proposal-for-shielded-assets-zsa-uda-for-defi-on-zcash/40520). Since ZSAs are a significant change to the Zcash protocol and its consensus rules, ZSAs should be included in the scope of the major audits of the network upgrade their implementation becomes a part of. It's always better to have more eyes on the code, so I plan to audit ZSAs myself as well.

### 8. Zephyr

[Zephyr](https://forum.zcashcommunity.com/t/project-zephyr-update-march22/41118)
is a project to build a metamask-like browser extension for Zcash. While the
project is still unfinished, I believe that support for Zcash in the browser is
on the critical path to adoption, so this is an important project to support.
Additionally, in-browser security models are complicated and error-prone, so it
will be worthwhile to take an early look.

### 9. Live Infrastructure

The Zcash ecosystem depends on live running infrastructure such as various
lightwalletd instances, the community forums, block explorers, and more. Zcash
users' security depends on the security of this infrastructure. Throughout the
year I will be offering security review and penetration tests to the maintainers
of this infrastructure.

### 9. Other

Several smaller-yet-popular projects like [Free2z](https://free2z.cash/docs/)
and [ZECpages](https://zecpages.com/z/all) are also deserving of security
review. Time permitting, I would like to perform some shorter audits of projects
like these alongside the longer audits listed above.

If you don't see your project on this list, but you feel like you could benefit
from an audit in 2023, please [reach out]({{<ref "contact.md">}}) and I will do
my best to accommodate you!

## Security Side-Projects

In addition to security audits, there are a number of "side projects" I hope to
work on to support Zcash's security.

### Office Hours and Other Security Assistance for Projects

Anyone working in the Zcash ecosystem can now set up a private meeting with me
using [my Calendly](https://calendly.com/zecsec/zcash-security-meeting).

The kind of support projects can get this way includes:

- Getting early security feedback on designs and code.
- Help with responding to vulnerability reports.
- Incident response for attacks on infrastructure.
- Secure UX review of API designs and user interfaces.
- Getting answers to technical questions about the Zcash protocol.
- Anything else security- or -privacy-related, really!

In addition to supporting Zcash projects themselves, I will also be available
to the Zcash Grant Committee to support their decision-making by providing
technical or security-related input.

### Wallet Edge Cases

A recommendation I've made in all of my wallet audits so far is to implement
better integration testing of wallets' state management in the face of edge
cases like reorgs, transactions failing to be mined, and other rare-but-possible
occurrences. Bugs in handling these kinds of edge cases could leave wallets in
invalid states, causing them to break or confuse users about how much funds they
have.

The first step towards improving wallet testing is to enumerate all such edge
cases. With a complete list of edge cases, the quality of existing wallet tests
can be measured by determining which edge cases are covered and which are not. 

In 2023 I hope to release a nearly-complete edge case list, which can then be
used by wallet authors to improve their tests.

### Consensus Rule Labeling

A security engineer entering the Zcash ecosystem for the first time is faced
with the daunting task of *finding* the code that implements Zcash's consensus
rules. As a result, a large portion of audit time is used inefficiently, spent
finding and understanding consensus rule code.

The efficiency of future consensus rule audits can be improved by labeling the
locations of all consensus rules in the code. Electric Coin Company has already
[started on this project](https://github.com/zcash/zcash/pull/5912), and in 2023
I hope to advance it by labeling more consensus rules and [writing a linter for
the rule labeling format](https://github.com/zcash/zcash/issues/6011).

### Secure Messaging Over Memos

A popular use case for Zcash's memo field is sending messages. However, Zcash's
memo field currently lacks several properties that are required for secure
messaging. For example, it is not signed, so wallets cannot be sure of messages'
origins, and it is not forward-secure, so if keys are compromised all past
messages can be decrypted.

In 2023, I hope to produce some design sketches that, if implemented, would make
it easier to build Signal-like secure messaging on top of Zcash. This project is
also a good candidate for a Zcash Community Grant RFP; I will be available to
assist the grant committee with writing up an RFP, if desired.

## What's Left Out?

In my recently-published [Zcash Ecosystem Security Overview]({{<ref
"overview.md">}}) page, I laid out a list of big picture security and privacy
challenges for Zcash. The roadmap above touches on some of them, however there
are several that I *probably won't* have dedicated time for in 2023. They
deserve to be highlighted anyway:

### Scalable Privacy for Wallets

A challenge faced by all cryptocurrencies that aim to offer strong, formal
privacy guarantees is: how can wallets' find their funds and make their funds
spendable quickly and efficiently?

At present, Zcash uses "trial decryption", where the wallet must try to decrypt
every transaction on the blockchain to find the ones that belong to it. There are many
alternatives to this design with varying levels of privacy and scalability.
I've surveyed them in my post, [Scalable Private Money Needs Scalable Anonymous Messaging]({{<ref "scalable-private-money-needs-scalable-private-messaging.md">}}).

### Unintentional and/or Forced Use of Transparent Transactions

Usage of transparent transactions on the Zcash blockchain remains high.
Transparent addresses and transactions offer users the ability to transact
transparently, with consent, whenever they wish to do so. However, the high
transparent usage might be a sign that some users misunderstand the privacy
level provided by transparent transactions or that users are forced into making
parts of their transactions transparent, i.e. by third-parties who do not fully
support shielded addresses. 

I know of at least one anecdote where someone put themselves at risk by using a
transparent address, because they thought "Zcash is private."

In my view, this problem should be tackled with (a) research into how
frequently users misunderstand the privacy properties of using transparent
addresses, (b) UX design within wallets that communicates privacy levels clearly
and simply, (c) support requests from the community to third parties and extra
engineering effort to increase shielded adoption, (d) an eventual removal of
transparent addresses, replaced by the use of viewing keys.

### Mitigating C++ Memory Corruption Bug Risk in Zcashd

The main fullnode implementation of Zcash is written in C++, which puts it at
risk of entire classes of security vulnerabilities that cannot exist within
projects that are written in safer languages, like Rust. Deprecating the legacy
``zcashd`` codebase should be a priority, to be replaced by ``zebra``. These
risks could also be mitigated with better fuzzing of ``zcashd``'s code, but it's
probably better to get rid of the C++ code entirely.

## Conclusion

I'm looking forward to a productive year of security audits in 2023. Along with
the audits, I'm planning several side projects in support of Zcash's security
like writing up better guidance for wallet testing, being more available to all
projects in office hours, working to label consensus rules, and exploring
improvements to the memo field for better secure messaging.

Of course, all of these plans are subject to change in case new higher-priority
risks or projects appear, but this should give the community a good sense for
what to expect in the coming year.

Let's make the new year Zcash's best year yet!