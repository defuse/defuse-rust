---
title: "YWallet Audit Results Published"
date: 2023-01-03T07:00:00-07:00
author: "Taylor Hornby"
draft: false
---

In October of last year, I reviewed [YWallet](https://ywallet.app/) for security
and privacy issues. This was the first audit I performed for the Zcash Ecosystem
Security grant.

Today, the final report is being made available to the Zcash community at the
link below.

The audit found one high-severity issue, two medium-severity issues, and several
low-severity issues. At this point in time, all issues have been remediated or
deemed to be safe to de-prioritize relative to other work.

The high-severity issue was a problem with the way the wallet stored users'
contact lists in transaction memos. An attacker who knew a user's address could
modify the user's contact list by sending them specially-crafted memos.  This
made it possible to carry out a man-in-the-middle between two users using
YWallet to chat with each other. The problem has been mitigated by only allowing
contact list updates from memos in transactions that were signed by the same
wallet. See the full report for details of the medium- and low-severity issues.

The report highlights the general need for a memo signing standard as well as a
more-comprehensive suite of tests for Zcash wallets. These are priorities in my
[2023 roadmap]({{<ref "zecsec-roadmap-for-2023.md">}}).

Thanks to Ywallet's author hanh for quick feedback on the report and fast bug fixes.

[**YWallet Security and Privacy Analysis Report (PDF)**](/audits/YWalletAuditReport-FINALv3.pdf)
