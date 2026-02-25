---
title: "ZecSec's Q1 2023 Transparency Report"
date: 2023-04-18T01:00:00-07:00
author: "Taylor Hornby"
draft: false
---

The ZecSec project publishes quarterly transparency reports in order to ensure
accountability and to help the Zcash community understand how its funds are
being spent. This is the report for Q1 of 2023.

Note that some of these reports may be delayed, and some information may be
redacted or slightly modified, in order to prevent the disclosure of unresolved
security bugs.

The current 
[grant](https://forum.zcashcommunity.com/t/zcash-ecosystem-security-lead/42090)
allows me to bill $1000 USD per day, up to a maximum of $17,000 per month, up to
12 total months. The sections below break down and explain my invoices for Q1
2023. The previous transparency report, for Q4 2022, can be found
[here](https://zecsec.com/posts/2022-q4-transparency-report/).

## January

In January, the bulk of my time went into a security audit of
[lightwalletd](https://github.com/zcash/lightwalletd). I also did a quick review
of [ZGo](https://zgo.cash/) and did some research on secure messaging
cryptography, thinking about how Zcash's memo field could be extended to better
support messaging use cases.

{{<table "table table-striped table-bordered">}}
| Days      | Description                                                                                       |
|-----------|---------------------------------------------------------------------------------------------------|
| 12        | Security audit of lightwalletd                                                                    |
| 1         | Secure messaging cryptography research                                                            |
| 2         | Quick security review of ZGo                                                                      |
| 2         | Miscellaneous time including office hours, PR review, and preparing the last transparency report  |
{{</table>}}

Total days: 17, Total paid: $17,000.

## February

In February, the main item I spent time on was thinking about how we could
[change the Zcash protocol to make transaction syncing more performant and
scalable](https://zecsec.com/posts/making-zcash-light-wallets-faster-and-more-private/).
I was also put in touch with some SGX/ORAM researchers interested in using the
technology to help solve Zcash's performance issues, which led me to writing a
[risk analysis of SGX and other Trusted Execution
Environments](https://zecsec.com/posts/risk-analysis-of-intel-sgx-and-other-tees/).

{{<table "table table-striped table-bordered">}}
| Days      | Description                                                                                       |
|-----------|---------------------------------------------------------------------------------------------------|
| 2         | lightwalletd audit remediation coordination/assistance                                            |
| 3         | SGX / TEE / ORAM research leading to the [risk analysis blog post](https://zecsec.com/posts/risk-analysis-of-intel-sgx-and-other-tees/) |
| 6         | [Scalable transaction detection protocol design](https://zecsec.com/posts/making-zcash-light-wallets-faster-and-more-private/) |
| 3         | Quick security audit of free2z                                                                    |
| 3         | Helping investigate disclosed [memory exhaustion bugs](https://electriccoin.co/blog/new-releases-remediate-memory-exhaustion-vulnerability-in-zcash/) in zcashd |
{{</table>}}

Total days: 17, Total paid: $17,000.

## March

In March, I audited [Zondax's shielded hardware wallet
code](https://github.com/Zondax/ledger-zcash). I also reviewed the ZIPs for
[Zcash Shielded Assets](https://github.com/zcash/zips/pull/680) (note that I
missed [a
bug](https://forum.zcashcommunity.com/t/grant-update-zcash-shielded-assets-monthly-updates/41153/42?u=earthrise)!).

{{<table "table table-striped table-bordered">}}
| Days      | Description                                                                                       |
|-----------|---------------------------------------------------------------------------------------------------|
| 12        | Security audit of Zondax's shielded Zcash Ledger app code                                         |
| 1         | Researching Identity-Based Encryption for scalable protocol designs                               |
| 3         | ZSA ZIPs review                                                                                   |
| 1         | Office hours and other miscellaneous items                                                        |
{{</table>}}

Total days: 17, Total paid: $17,000.

## Conclusion

In Q1 of 2023, I completed two major audits (of lightwalletd and Zondax's
shielded hardware wallet), several quick audits (of ZGo, free2z, and the ZSA
ZIPs), and published research on a scalable protocol design and an analysis of
the risks of using SGX/ORAM to fix Zcash's performance issues.

In total, I invoiced 51 days and received $51,000 paid in ZEC.

If you have questions about the work that was done or have suggestions for
future priorities, you can either email me at zecsec@defuse.ca or reply on [the
grant's forum
thread](https://forum.zcashcommunity.com/t/zcash-ecosystem-security-lead/42090).