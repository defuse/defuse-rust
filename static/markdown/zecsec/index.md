# ZecSec: Zcash Ecosystem Security

This page serves as an overview of the Zcash ecosystem from a security
auditor's point of view. It lists all of the projects that are intended to fall
under the scope of the [ZecSec project](https://forum.zcashcommunity.com/t/zcash-ecosystem-security-lead/42090), as well as past audit reports, notable
security bugs, and open security/privacy challenges in the Zcash ecosystem. You
can think of this page as "a security auditor's guide to Zcash."

## Posts

- January 5, 2024 &ndash; [ZGo Security Audit Results](/zecsec/zgo-security-audit.htm)
- September 24, 2023 &ndash; [The Future of Zcash Ecosystem Security](/zecsec/future-of-zcash-ecosystem-security.htm)
- September 23, 2023 &ndash; [Security Audit of zecwallet-lite-cli](/zecsec/zecwallet-lite-cli-security-audit.htm)
- September 21, 2023 &ndash; [Results of Auditing Hanh's Shielded Zcash Ledger App](/zecsec/security-audit-of-hanhs-ledger-app.htm)
- September 21, 2023 &ndash; [Mitigating 51% Attack Risk on the Zcash Network](/zecsec/zcash-51-percent-attack.htm)
- September 14, 2023 &ndash; [Free2Z Security Audit Results](/zecsec/free2z-security-audit.htm)
- August 11, 2023 &ndash; [If you used libbitcoin-explorer (bx) to generate your seed phrase, rotate it ASAP!](/zecsec/milk-sad.htm)
- July 30, 2023 &ndash; [Security Engineering: Learning from Safety-Critical Disciplines](/security-engineering-learning-from-safety-critical-disciplines.htm)
- April 18, 2023 &ndash; [ZecSec's Q1 2023 Transparency Report](/zecsec/2023-q1-transparency-report.htm)
- April 15, 2023 &ndash; [A Simple Threat Model for Zcash Shielded Hardware Wallets](/zecsec/threat-model-for-zcash-hardware-wallets.htm)
- March 10, 2023 &ndash; [Risk Analysis of Intel's SGX and Other TEEs](/zecsec/risk-analysis-of-intel-sgx-and-other-tees.htm)
- March 2, 2023 &ndash; [Making Zcash Light Wallets Faster and More Private](/zecsec/making-zcash-light-wallets-faster-and-more-private.htm)
- January 27, 2023 &ndash; [ZecSec's Q4 2022 Transparency Report](/zecsec/2022-q4-transparency-report.htm)
- January 3, 2023 &ndash; [ZecSec Roadmap for 2023](/zecsec/zecsec-roadmap-for-2023.htm)
- January 3, 2023 &ndash; [YWallet Audit Results Published](/zecsec/ywallet-audit-published.htm)
- December 29, 2022 &ndash; [Security Audit Process](/zecsec/security-audit-process.htm)
- November 15, 2022 &ndash; [Scalable Private Money Needs Scalable Anonymous Messaging](/zecsec/scalable-private-money-needs-scalable-private-messaging.htm)
- October 28, 2022 &ndash; [October Update: Ywallet audited, and what's next?](/zecsec/october-update.htm)
- October 13, 2022 &ndash; [Hello, World!](/zecsec/my-first-post.htm)

## Audit Reports

- [YWallet Security and Privacy Analysis Report (PDF)](/zecsec/audits/YWalletAuditReport-FINALv3.pdf)
- [Security Audit of zecwallet-lite-cli (PDF)](/zecsec/audits/zecwallet-lite-cli-audit-report-v2.pdf)
- [Shielded Zcash Ledger App Audit Report (PDF)](/zecsec/audits/zcash-ledger-audit-report-v2.pdf)
- [Free2Z Audit Report (PDF)](/zecsec/audits/Free2Z%20Mini%20Audit-Final.pdf)
- [Security and Privacy Analysis of ZGo (PDF)](/zecsec/audits/ZGo-Security-Audit-v1.1.pdf)

## Highlighting Open Problems and Challenges

There are several "big picture" security and privacy challenges for Zcash that
are on ZecSec's radar. These are not necessarily being worked on as part of
ZecSec, but are being spotlighted here as our recommendations for ecosystem-wide
priorities.

### Scalable Privacy For Wallets

A challenge faced by all cryptocurrencies that aim to offer strong, formal
privacy guarantees is: how can wallets' find their funds and make their funds
spendable quickly and efficiently?

At present, Zcash uses "trial decryption", where the wallet must try to decrypt
every transaction on the blockchain to find the ones that belong to it. There are many
alternatives to this design with varying levels of privacy and scalability.
We've surveyed them in our post, [Scalable Private Money Needs Scalable Anonymous Messaging](/zecsec/scalable-private-money-needs-scalable-private-messaging.htm).

### Unintentional and/or Forced Use of Transparent Transactions

Usage of transparent transactions on the Zcash blockchain remains high.
Transparent addresses and transactions offer users the ability to transact
transparently, with consent, whenever they wish to do so. However, the high
transparent usage might be a sign that some users misunderstand the privacy
level provided by transparent transactions, or that users are forced into making
parts of their transactions transparent by third-parties who do not fully
support shielded addresses.

I know of at least one anecdote where someone put themselves at risk by using a
transparent address, because they thought "Zcash is private."

In our view, this problem should be tackled with (a) research into how
frequently users misunderstand the privacy properties of using transparent
addresses, (b) UX design within wallets that communicates privacy levels clearly
and simply, (c) support requests from the community to third parties and extra
engineering effort to increase shielded adoption, (d) an eventual removal of
transparent addresses, replaced by the use of viewing keys.

### Secure Messaging with the Memo Field

A popular use case for Zcash's memo field is sending messages. However, Zcash's
memo field currently lacks several properties that are required for secure
messaging. For example, it is not signed, so wallets cannot be sure of messages'
origins, and it is not forward-secure, so if keys are compromised, all past
messages can be decrypted.

We would like to see the memo field extended with more features so that it is
easy to build messengers on top of it with Signal-like security properties.

### Mitigating C++ Memory Corruption Bug Risk in Zcashd

The main fullnode implementation of Zcash is written in C++, which puts it at
risk of entire classes of security vulnerabilities that cannot exist within
projects that are written in safer languages, like Rust. Deprecating the legacy
`zcashd` codebase should be a priority, to be replaced by `zebra`. These
risks could also be mitigated with better fuzzing of `zcashd`'s code, but it's
probably better to get rid of the C++ code entirely.

### Easing Consensus Rule Security Review

A security engineer entering the Zcash ecosystem for the first time is faced
with the daunting task of *finding* the code that implements Zcash's consensus
rules. As a result, a large portion of audit time is used inefficiently, spent
finding and understanding consensus rule code.

The efficiency of future consensus rule audits can be improved by labeling the
locations of all consensus rules in the code. Electric Coin Company has already
[started on this project](https://github.com/zcash/zcash/pull/5912). Labeling
more consensus rules is on [ZecSec's 2023 roadmap](/zecsec/zecsec-roadmap-for-2023.htm), but hopefully all maintainers of consensus
rule code will be able to contribute to this project.

## Projects In Scope (alphabetical order)

ZecSec aims to eventually provide security support for all Zcash-tangential
projects that are currently used, are in development, or run important
infrastructure for the community. Listed below are all of the projects that are
potentially in scope.

Note that security support has not yet been "rolled out" to most of these
projects yet!  For more details on prioritization, see our [2023 roadmap](/zecsec/zecsec-roadmap-for-2023.htm). If your Zcash-related project is missing
from the list below, [send us a note](mailto:zecsec@defuse.ca).

### Arti

Arti is a Tor library written in pure Rust. It may soon be adopted by various
Zcash wallets to improve wallet privacy in various ways.

#### Approved Grants

- https://forum.zcashcommunity.com/t/arti-a-pure-rust-tor-implementation-for-zcash-and-beyond/38776

#### Proposed Grants

- https://forum.zcashcommunity.com/t/arti-year-ii-funding-plan-onion-services-and-more/41387

### CoinPayments Zcash Integration

A grant was funded to add Zcash support into CoinPayments. Unfortunately the work was never picked up by CoinPayments:

> Hi everyone, CoinPayments has informed us that they are scrapping their plans for the new platform which was the driver behind this grant to add shielded support. They are going to continue to support Zcash payments on the platform, but this means the work @hanh has done toward the integration will not be implemented.

The work done for this grant is to be repurposed to support BTCPay instead.

#### Approved Grants

- https://forum.zcashcommunity.com/t/coinpayments-integration/39094 (superseded by the BTCPay grant)

### Edge

The [Edge Wallet](https://edge.app/) supports shielded Zcash.

### Electric Coin Company Projects

Electric Coin Company maintains many essential parts of the Zcash ecosystem:

- [zcashd](https://github.com/zcash/zcash)
- [librustzcash](https://github.com/zcash/librustzcash) (and many of its cryptographic dependencies)
- [lightwalletd](https://github.com/zcash/lightwalletd)
- [iOS Wallet SDK](https://github.com/zcash/ZcashLightClientKit)
- [Android Wallet SDK](https://github.com/zcash/zcash-android-wallet-sdk)
- ["Secant" Wallet iOS](https://github.com/zcash/secant-ios-wallet)
- ["Secant" Wallet Android](https://github.com/zcash/secant-android-wallet)
- [Wallet App Threat Model](https://zcash.readthedocs.io/en/latest/rtd_pages/wallet_threat_model.html)
- and many more...

### ElectrumZ

A grant was funded to make a fork of the [Electrum](https://electrum.org/)
wallet with support for Zcash.

#### Approved Grants

- https://forum.zcashcommunity.com/t/electrum-fork-for-zcash/30994

### Elemental ZEC - Zcash UI Component Kit and Payment

This grant aims to build frontend code making it easier for merchants to
interact with Zcash. They are also building a watch-only wallet in node.js to
serve as a backend in place of proper BTCPay integration.

[Project website (docs)](https://elementalzcash.com/)

#### Approved Grants

- https://grants.zfnd.org/proposals/648795356-elemental-zec-zcash-ui-component-kit-and-payment-processor

### Free2Z

[Free2Z](https://free2z.cash) is a blogging and donation platform built around Zcash.

#### Approved Grants

- https://zcashgrants.org/gallery/25215916-53ea-4041-a3b2-6d00c487917d/24075557/

### FROST

[FROST](https://zfnd.org/frost/) is a multiparty signature scheme, [in the
process of being standardized](https://zfnd.org/frost/), that one day can be
used to implement shielded multisig for Zcash. Its development is supported by
the Zcash Foundation.

### Ledger support for Transparent Zcash

Ledger maintains support for transparent Zcash in their hardware wallet offerings. See "Zondax" below for support for shielded Zcash on Ledger.

### Moeda.casa

Moeda.casa is some kind of crypto<->fiat exchange platform targeting Brazil.
It's unclear what their current status is (the website is throwing SSL errors).

#### Approved Grants

- https://grants.zfnd.org/proposals/1868138724-moeda-casa-smart-brazilian-fiat-to-crypto-over-zcash

### Nighthawk

Nighthawk is an Android and iOS app for Zcash, originally built off of Electric
Coin Company's demo ("dogfooding") app codebases, which uses Electric Coin
Company's SDKs.

Nighthawk has a number of components that are security-relevant:

- Production lightwalletd instances for wallets to connect to.
- [zcashblockexplorer.com](https://zcashblockexplorer.com) is running [nighthawk-apps/zcash-explorer](https://github.com/nighthawk-apps/zcash-explorer)
- [nighthawk-apps/nighthawk-wallet-android](https://github.com/nighthawk-apps/nighthawk-wallet-android) is the android wallet.
- [nighthawk-apps/zcash-ios-wallet](https://github.com/nighthawk-apps/zcash-ios-wallet) is the iOS wallet.
- Various forks of Electric Coin Company repos: [nighthawk-apps/bip39](https://github.com/nighthawk-apps/bip39), [nighthawk-apps/zcash-android-wallet-sdk](https://github.com/nighthawk-apps/zcash-android-wallet-sdk), [nighthawk-apps/lightwalletd](https://github.com/nighthawk-apps/lightwalletd).

#### Approved Grants

- https://forum.zcashcommunity.com/t/nighthawk-wallet-design-development-grant/39017
- https://forum.zcashcommunity.com/t/2-years-of-lightwalletd-infra-hosting-maintenance/38126
- https://forum.zcashcommunity.com/t/zcash-block-explorer-grant/38141
- https://grants.zfnd.org/proposals/68882602

### Oblivious Message Retrieval (OMR)

[Oblivious Message Retrieval](https://eprint.iacr.org/2021/1256.pdf) is a
homomorphic-encryption based approach to the "trial decryption" transaction
scanning problem. A grant was awarded to build a prototype of the system.

#### Approved Grants

- https://forum.zcashcommunity.com/t/oblivious-message-retrieval/40715

### Payment Gateway with BTCPay

[BTCPay](https://btcpayserver.org/) is a self-hosted server for accepting
cryptocurrency payments. A grant was approved to integrate Zcash.

#### Approved Grants

- https://forum.zcashcommunity.com/t/payment-gateway-btc-pay/40207

### paywithz.cash

A list of stores and services that accept Zcash is maintained at [paywithz.cash](https://paywithz.cash/).

### Proof of Stake Design

Electric Coin Company has proposed that Zcash transitions to Proof of Stake (PoS) in
the near future. It is an open question which PoS design will be selected and
what its security properties will be.

### Quiet (formerly known as Zbay)

[Quiet](https://www.tryquiet.org/), formerly known as Zbay, was a project that
attempted to use the memo field as a secure communication layer. The project
ended up moving to Tor out of a need for better performance.

#### Approved Grants

- https://forum.zcashcommunity.com/t/zbay-applied-for-a-zf-grant-any-feedback-on-our-proposal/36584

### react-native-zcash

The ZcashLightClientKit SDK is packaged into a React Native library [here](https://github.com/EdgeApp/react-native-zcash).

### renZEC

Ren is a project that bridges between blockchains, producing, for example,
renZEC as an ERC20 token representing units of ZEC on the Ethereum Blockchain. A
[currently-centralized bridge](https://github.com/renproject/ren/wiki/Phases)
holds funds and is used to maintain the peg between chains.  A grant was
approved to bootstrap liquidity for trading renZEC.

#### Approved Grants

- https://grants.zfnd.org/proposals/118814097-bootstrapping-liquidity-for-renzec-on-binance-smart-chain

### Stagnum (Zcash Node Hardware)

Stagnum is a team that was approved for a grant to build a hardware device for easily running a Zcash fullnode.

#### Approved Grants

- https://zcashgrants.org/gallery/25215916-53ea-4041-a3b2-6d00c487917d/31829236/

### Telegram Anti-Scam Bot

This is a bot that can be used within Telegram channels to kick out accounts
that are obvious scammers. It uses simple matching against the accounts'
usernames to kick out accounts that follow common scam patterns.

#### Approved Grants

- https://forum.zcashcommunity.com/t/zcash-anti-scam-telegram-bot-grant/42845

### Thorchain Integration

Thorchain is a decentralized exchainge (DEX). A grant was approved to integrate
Zcash into Thorchain and their wallets.

#### Approved Grants

- https://forum.zcashcommunity.com/t/zcash-thorchain-integration-grant/39564

### Trezor Support for Transparent & Shielded Zcash

Trezor has long maintained support for transparent Zcash in their hardware
wallets. More recently, they were awarded a grant to integrate with shielded
Zcash.

The code for shielded Zcash is in [this pull request](https://github.com/trezor/trezor-firmware/pull/2472).

#### Approved Grants

- https://forum.zcashcommunity.com/t/trezor-support-for-zcash-shielded-transactions/39420

### Ywallet

Ywallet is a Zcash and [Ycash](https://y.cash/) wallet built independently from the Electric Coin Company SDKs.

### Approved Grants

- https://forum.zcashcommunity.com/t/ywallet-warp-sync/42620
- https://forum.zcashcommunity.com/t/orchard-and-ua-for-ywallet/43061
- https://forum.zcashcommunity.com/t/cold-wallet/38664

### Zboard

Zboard is a reddit-like social media platform built on Zcash's memo field. At
the time of writing, the website is not functional, presenting an error on the
homepage.

#### Approved Grants

- https://grants.zfnd.org/proposals/1854600060-zboard-global-decentralized-news-hub

### Zcash Blockchain Infrastructure Grant

This is a grant to write containers and deployment infrastructure for Zcash full
notes. The project's github is [zbitech](https://github.com/zbitech).

#### Approved Grants

- https://forum.zcashcommunity.com/t/grant-idea-zcash-blockchain-infrastructure-zbi/40514

### Zcash Community Forums

The [Zcash Community Forums](https://forum.zcashcommunity.com/) are hosted by
the Zcash Foundation, they are a central hub of discussion among the Zcash
community.

### Zcash Foundation Projects

The Zcash Foundation maintains several projects central to the Zcash ecosystem:

- [ZcashFoundation/zebra](https://github.com/ZcashFoundation/zebra)
- [ZcashFoundation/frost](https://github.com/ZcashFoundation/frost)

### Zcash Media

Zcash Media is a project by 37 Laines (37L) that produces interviews and
educational videos on the topic of Zcash.

#### Approved Grants

- https://forum.zcashcommunity.com/t/zcash-media-launch-plan/41138

#### Proposed Grants

- https://forum.zcashcommunity.com/t/zcash-media-2022-2023/42246

### Zcash Observatory

Zcash Observatory is a project to augment zcashd with code for observing and
reporting telemetry on the p2p network topology and other data.

#### Approved Grants

- https://grants.zfnd.org/proposals/21786689-zcash-observatory

### Zcash Protocol Specification

The [Zcash Protocol Specification](https://zips.z.cash/protocol/protocol.pdf)
describes the Zcash protocol and its consensus rules. It is maintained by
Electric Coin Company.

### ZECPages

[ZECPages](https://zecpages.com) is a public message board built on Zcash's memo
field. It runs a production lightwalletd server at
`lightwalletd.zecpages.com:443` and its code is on github at
[michaelharms6010/zecpages](https://github.com/michaelharms6010/zecpages).
Michael Harms has also received a grant to run a testnet faucet, linked below.

#### Approved Grants

- https://forum.zcashcommunity.com/t/1-year-of-zecpages-servers/38823
- https://grants.zfnd.org/proposals/2008792221-zecpages-testnet-faucet-app-development-1-year-infra

### ZecWallet and ZecWallet-Lite

[ZecWallet](https://zecwallet.co/fullnode.html) is a UI+fullnode package for
Windows, Mac, and Linux. [ZecWallet-Lite](https://zecwallet.co/index.html) is a
Zcash light wallet. The authors of the wallets maintain [a fork of
lightwalletd](https://github.com/adityapk00/lightwalletd) with additional
features such as a price API and transaction spam filtering. They also run
production lightwalletd instances for ZecWallet-Lite wallets.

#### Approved Grants

- https://forum.zcashcommunity.com/t/add-orchard-to-zecwallet-mobile-and-desktop-apps/42134
- Many others (to be listed!)

### ZecWear

[ZecWear](https://zecwear.com) is a Zcash clothing and merch store.

### ZEGA

ZEGA describes itself as "a command line encrypted-file-sharing CLI using
ZecWallet light client." The code lives at
[wh00hw/ZEGA](https://github.com/wh00hw/ZEGA).

### Zeme Teme

[Zeme Team](https://zeme.team) is a website for sharing Zcash-related memes.

#### Approved Grants

- https://forum.zcashcommunity.com/t/zeme-team-marketing-made-viral-for-zcash/38912

### Zemo

Zemo is a proposed messaging app that would be built on the Zcash blockchain.

#### Proposed Grants

- https://forum.zcashcommunity.com/t/zemo-your-web3-inbox/41063/15

### Zephyr

Zephyr is a still-in-development metamask-like browser extension for Zcash.

#### Approved Grants

- https://forum.zcashcommunity.com/t/zephyr-a-metamask-style-browser-extension-for-zcash/39112
- https://forum.zcashcommunity.com/t/project-zephyr-update/40657 (update thread)

### ZGo

[ZGo](https://zgo.cash/) is a point-of-sale tool for Zcash. The project's
twitter account is [ZGoCashApp](https://twitter.com/ZGoCashApp).

#### Approved Grants

- https://forum.zcashcommunity.com/t/zgo-the-zcash-register/41885

### Ziggurat

Ziggurat is a security-focused project investigating Zcash's peer-to-peer
network. Their main offering is a fuzzer of the p2p protocol which has proven
successful at finding bugs and inconsistencies in the zcashd node's behavior.
The project is being expanded from single-node testing into a tool that can
actually crawl the network, and their latest grant proposes "red team" tests of
testnet.

### Approved Grants

- https://forum.zcashcommunity.com/t/ziggurat-the-zcash-network-stability-framework/38758
- https://forum.zcashcommunity.com/t/ziggurat-2-0/41201
- https://forum.zcashcommunity.com/t/ziggurat-3-0/43350

### ZingoLabs' wallet (zingolabs on github):

[ZingoLabs](https://github.com/zingolabs) is a team in the process of developing
Zcash software, including a wallet. Their notable GitHub repos are:

- [zingolib](https://github.com/zingolabs/zingolib) -- a library and command-line interface
- [zingo-mobile](https://github.com/zingolabs/zingo-mobile) -- the mobile apps themselves.
- [zingo](https://github.com/zingolabs/zingo) -- a fork of zecwallet-lite

#### Approved Grants

- https://forum.zcashcommunity.com/t/implement-orchard/42706

### ZIPs

Changes to the Zcash protocol are described using [Zcash Improvement Proposals](https://zips.z.cash/).

### Zondax (Zcash Ledger App)

Zondax is the team building shielded support for Ledger hardware wallets. Their
code lives in the [zondax](https://github.com/zondax) GitHub Org, with the main
implementation being in
[zondax/ledger-zcash](https://github.com/zondax/ledger-zcash).

#### Approved Grants

- https://forum.zcashcommunity.com/t/hw-wallets-z-transactions/34867/24
- https://grants.zfnd.org/proposals/310598051-new-zcash-ledger-app-integration

### ZSAs

Zcash Shielded Assets (ZSAs) is an extension to the Zcash protocol that, once
implemented, will allow shielded transactions to support different asset types,
e.g. BTC-on-ZEC. The designs and implementations are being provided by QEDIT, funded through a grant.

Links to the draft ZIP specifications and the implementation code can be found in [this forum comment](https://forum.zcashcommunity.com/t/grant-update-zcash-shielded-assets-monthly-updates/41153/18?u=earthrise).

#### Approved Grants

- https://forum.zcashcommunity.com/t/grant-update-zcash-shielded-assets-monthly-updates/41153
- https://zcashgrants.org/gallery/25215916-53ea-4041-a3b2-6d00c487917d/33106640/

## List of Security Audits

### 2016

The initial implementation of Zcash was audited by NCC Group and Coinspect. The audits
were announced [here](https://electriccoin.co/blog/auditing-zcash/), the
completed reports are linked
[here](https://electriccoin.co/blog/audit-results/), and some discussion of the
issue mitigations is
[here](https://electriccoin.co/blog/new-beta-release-audit-mitigations-and-bug-fixes/).

Zcash's first parameter generation ceremony was also [audited by NCC Group](https://electriccoin.co/blog/ceremony-audit-results/).

### 2017

N/A

### 2018

In 2018, audits were performed by Least Authority, Coinspect, NCC Group,
Kudelski Security, and QEDIT. The audit results are discussed
[here](https://electriccoin.co/blog/2018-security-audit-results-in-detail/), but
unfortunately the links to many of the audit reports are now broken.

Least Authority's audit reports are available here: [summary](https://medium.com/least-authority/zcash-security-audit-a72945842f8e), [Overwinter audit](https://leastauthority.com/wp-content/uploads/2019/08/LeastAuthority-Zcash-Implementation-Analysis-and-Overwinter-Specification.pdf), [Overwinter+Sapling audit](https://leastauthority.com/static/publications/LeastAuthority-Zcash-Overwinter+Sapling-Specification-Final-Audit-Report.pdf), [Sapling RPC audit](https://leastauthority.com/static/publications/LeastAuthority-Zcash-Sapling-Implementation-RPC-Interface-Updated-Audit-Report.pdf).

QEDIT's audit of Sapling is available [here](https://raw.githubusercontent.com/QED-it/sapling-audit/master/sapling-audit-report.pdf).

### 2019

In 2019, [TrailOfBits audited ZecWallet](https://github.com/trailofbits/publications/blob/master/reviews/zecwallet.pdf) and [Zcash Blossom was audited by Coinspect and NCC Group](https://electriccoin.co/blog/blossom-network-upgrade-and-wallet-security-audits/).

### 2020

In 2020, [TrailOfBits audited Zcash Heartwood](https://electriccoin.co/blog/heartwood-security-assessment-turns-up-no-major-issues/), [NCC Group audited Zcash Canopy](https://electriccoin.co/blog/canopy-security-assessment-complete/), and [Electric Coin Company performed an internal audit of their wallets](https://electriccoin.co/blog/ecc-wallet-threat-model-and-security-assessment-results/).

### 2021

In 2021, [NU5 was audited by NCC Group and QEDIT](https://electriccoin.co/blog/nu5-security-assessments-complete/).

### 2022

In 2022, [the design of the Halo2 proving system was audited by Mary Maller](https://z.cash/halo2-audit/).

ZecSec performed an audit of Ywallet (*report forthcoming*) and zecwallet-lite-cli (*report forthcoming*).

## Notable Security Bugs and Other Security Research

Secure systems can only be built by learning from mistakes. Here is a list of
notable bugs and other security research that we can learn from to make Zcash a
more robust platform.

### Viewing key disclosure bug

The ECC wallets and Nighthawk accidentally leaked the wallet's viewing key where
the reply address should been within the "Reply-To" component of the memo field.

[Disclosure Blog Post](https://electriccoin.co/blog/privacy-leak-bug-discovered-in-nighthawk-and-ecc-wallets/)

### Counterfeiting vulnerability

A mistake in the BCTV14 paper, which carried over to its implementation, made it
possible to violate the soundness property of the zero-knowledge proving system
originally used by Zcash. If this bug was exploited, it would have allowed for
the forgery of Zcash funds, limited by the [turnstile
design](https://electriccoin.co/blog/turnstile-enforcement-against-counterfeiting/)
that was built as a defense-in-depth measure against these kinds of attacks.

This bug was assigned CVE-2019-7167.

[Disclosure Blog Post](https://electriccoin.co/blog/zcash-counterfeiting-vulnerability-successfully-remediated/)

### Side-Channel Attacks on Zcash Node Transaction Receivers

In an [academic paper](https://eprint.iacr.org/2020/220), Florian Tramèr, Dan
Boneh, and Kenneth G. Paterson explored the use of timing, traffic analysis, and
error-message side-channels to de-anonymize nodes in receipt of transactions.

They found that when a Zcash node successfully decrypted a transaction, but the
note commitment check failed, the node would send an error message back to the
attacker, indicating to the attacker that the node had the key required to
decrypt the transaction. Additionally, since the P2P message processing all
happened on the same thread, an attacker could measure the response time of a
"ping" message to determine if the note commitment check was happening; another
way to confirm the node had the correct decryption key. Through either of these
approaches, it was possible for an attacker to conclude that a known shielded
address belonged to the victim node.

The vulnerability in Zcash was assigned CVE-2019-16930, and there is a
corresponding [security
announcement](https://z.cash/support/security/announcements/security-announcement-2019-09-24/).

### Sapling Woodchipper

"[Sapling Woodchipper](https://saplingwoodchipper.github.io/)" is a name given
to the concept that as long as transaction fees are cheap, it is inexpensive to
fill up blocks on the Zcash blockchain. It was assigned CVE-2019-1636 by its
initial reporter.  It is currently being mitigated by the rollout of a new
transaction fee structure, specified in [ZIP-317](https://zips.z.cash/zip-0317).

### Faerie Gold Attack

In the original design of Zerocash, it was possible for an attacker to send a
victim user two notes with identical nullifiiers. As a result, the victim's
wallet would be tricked into thinking it could spend both notes, when in fact it
would only be able to spend one of them. The details of the attack and the fix are
[described in the Zcash protocol
specification](https://zips.z.cash/protocol/protocol.pdf#faeriegold).

### InternalH Collision Vulnerability

In the original [Zerocash paper](https://eprint.iacr.org/2014/349) that Zcash
was based on, a 128-bit hash was used to build a commitment scheme. This was
vulnerable to a birthday attack, which would have made it possible to
counterfeit funds. The bug was [identified and fixed prior to Zcash's
launch](https://electriccoin.co/blog/fixing-zcash-vulns/).

### ZecWallet-Lite TLS Authentication Bug

Sarah Jamie Lewis [discovered that a previous version of ZecWallet-Lite was not
properly authenticating TLS
connections](https://twitter.com/SarahJamieLewis/status/1236137690900258818), so
that its communications with the lightwalletd server could be intercepted by an
attacker.

### ZecWallet Nonce Reuse Replay Attack

ZecWallet [optionally uses a "wormhole" service to allow a companion smartphone app to connect to the user's full node](https://docs.zecwallet.co/android/). The encryption used by the wormhole protocol was vulnerable to a replay attack that allowed for nonce reuse. This was [reported by Sarah Jamie Lewis](https://twitter.com/SarahJamieLewis/status/1234585483482550273?s=20). See also [a further bug in the fix to the issue](https://github.com/ZcashFoundation/zecwallet/issues/243).

### Fuzzing `zcashd` with Kubernetes

Electric Coin Company engaged in a project to [fuzz `zcashd`'s C++ codebase in an
automated way using
Kubernetes](https://electriccoin.co/blog/fuzzing-zcash-with-kubernetes/).

### Miscellanious Bugs & Security Announcements

- https://electriccoin.co/blog/nu5-activation-and-halo-arc-release-delayed-for-remediation-of-consensus-bug/
- https://electriccoin.co/blog/security-announcement-2017-04-13/
- https://electriccoin.co/blog/security-announcement-2017-04-12/
- https://electriccoin.co/blog/security-announcement-2016-11-22/

### Academic Papers Analyzing Zcash

Zcash has been investigated in several academic works, linked below:

- [Privacy Aspects and Subliminal Channels in Zcash](https://dl.acm.org/doi/abs/10.1145/3319535.3345663)
- [Privacy and Linkability of Mining in Zcash](https://ieeexplore.ieee.org/abstract/document/8802711)
- [Blockchain Access Privacy: Challenges and Directions](https://ieeexplore.ieee.org/abstract/document/8425613)
- [Security and privacy of mobile wallet users in Bitcoin, Dash, Monero, and Zcash](https://www.sciencedirect.com/science/article/abs/pii/S1574119218307181)
- [A Look into Privacy-Preserving Blockchains](https://ieeexplore.ieee.org/abstract/document/9035235)
- [On the linkability of Zcash transactions](https://arxiv.org/abs/1712.01210) ([Electric Coin Company's response](https://electriccoin.co/blog/new-research-on-shielded-ecosystem/))
- [An Empirical Analysis of Anonymity in Zcash](https://www.usenix.org/conference/usenixsecurity18/presentation/kappos&lang=en)
- [Extending the Anonymity of Zcash](https://arxiv.org/abs/1902.07337)
- [Deanonymization and Linkability of Cryptocurrency Transactions Based on Network Analysis](https://ieeexplore.ieee.org/abstract/document/8806723)
- [Map-Z: Exposing the Zcash Network in Times of Transition](https://ieeexplore.ieee.org/abstract/document/8990796)
- [A Refined Analysis of Zcash Anonymity](https://ieeexplore.ieee.org/abstract/document/8993834)
- [Attacking Zcash For Fun and Profit](https://eprint.iacr.org/2020/627)
- [Anonymity Analysis of Bitcoin, Zcash, and Ethereum](https://ieeexplore.ieee.org/abstract/document/9389894)
- [An Analysis of Anonymity in the Zcash Cryptocurrency](https://deepblue.lib.umich.edu/handle/2027.42/143130)
- [Modeling the Block Verification Time of Zcash](https://ieeexplore.ieee.org/abstract/document/9583684)
- [A Comparative Study of Privacy-Preserving Cryptocurrencies: Monero and ZCash](http://www.dgalindo.es/mscprojects/sofie.pdf)
- [Exploring the use of Zcash cryptocurrency for illicit or criminal purposes](https://www.rand.org/pubs/research_reports/RR4418.html)
- [PING and REJECT: The Impact of Side-Channels on Zcash Privacy](https://crypto.stanford.edu/timings/pingreject.pdf)
- [A Review of Zcash as a Cryptocurrency Platform Aimed Towards Maintaining Privacy Between All Parties](https://static1.squarespace.com/static/553d790de4b08ceb08ab88fd/t/60022ff3652b4632bc8d632b/1610756084528/Zcash_SciPaper.pdf)

If you know of Zcash-related security research that's missing from this list, please [send us a note](mailto:zecsec@defuse.ca).

## Contact

**Email:** [zecsec@defuse.ca](mailto:zecsec@defuse.ca)

**Discord:** Taylor Hornby#2689

PGP Key:

```
-----BEGIN PGP PUBLIC KEY BLOCK-----

mDMEY2MF6hYJKwYBBAHaRw8BAQdAcydGuJEjjdC2PQqrfuwGoEevjJraixC+hbUe
N78tGnW0IFRheWxvciBIb3JuYnkgPHplY3NlY0BkZWZ1c2UuY2E+iJkEExYKAEEW
IQQX8Ob+vL++LE7u5BX00Y8PG75JEwUCY2MF6gIbAwUJCWYBgAULCQgHAgIiAgYV
CgkICwIEFgIDAQIeBwIXgAAKCRD00Y8PG75JE49tAQD6eHXdWuwCXw+01bwyFi3a
7BqG/slvHYcAVHuRCVdjFAEAgzzKxrAEGgxEoSGrqRKpAhBd4TYcRVli/SRRx+Pg
cwy4OARjYwXqEgorBgEEAZdVAQUBAQdAa3gkhLRD86YXufg0IvZEoWFyjAJQy+Ss
Ru8WrOybVVIDAQgHiH4EGBYKACYWIQQX8Ob+vL++LE7u5BX00Y8PG75JEwUCY2MF
6gIbDAUJCWYBgAAKCRD00Y8PG75JE1tlAP4/WZw4riaYu8REcxESsv9E8sF62Lk8
OgIPJuNo7aHkiQEApBqdgy+saFGIk0m+EzJ7SKbJF8/9brICEIneRzhc/As=
=cpn7
-----END PGP PUBLIC KEY BLOCK-----
```

PGP Key Fingerprint: `17F0 E6FE BCBF BE2C 4EEE  E415 F4D1 8F0F 1BBE 4913`
