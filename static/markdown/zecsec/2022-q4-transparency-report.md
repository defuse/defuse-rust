---
title: "ZecSec's Q4 2022 Transparency Report"
date: 2023-01-27T01:00:00-07:00
author: "Taylor Hornby"
draft: false
---

In order to ensure accountability and to help the Zcash community understand how
its funds are being spent, the ZecSec project will be posting quarterly
transparency reports. 

Note that some of these reports may be delayed, and some information may be
redacted, in order to prevent the disclosure of unresolved security bugs.

The currently-approved
[grant](https://forum.zcashcommunity.com/t/zcash-ecosystem-security-lead/42090)
allows me to bill $1000 USD per day, up to a maximum of $17,000 per month, up to
12 total months. The sections below break down and explain my invoices for Q4
2022.

## September & October

In September and October my focus was on auditing Ywallet. Ywallet was a
priority because it had not yet received any security review and it was one of
the only wallets that functioned in the face of the high transaction load
problem. The report from this audit is available [here]({{<ref
"ywallet-audit-published.md">}}).

{{<table "table table-striped table-bordered">}}
| Days      | Description                                                                                       |
|-----------|---------------------------------------------------------------------------------------------------|
| 1         | Setting up build environments, skimming over grants/projects, staying up to date on the forums.   |
| 1         | Reviewing various Zcash wallets for a potential DoS bug reported by ZingoLabs.                    |
| 8         | Auditing Ywallet's zcash-sync library.                                                            |
| 5         | Auditing Ywallet itself.                                                                          |
| 1         | Delivering the Ywallet audit report and answering questions.                                      |
| 0.5       | Setting up this website.                                                                          |
| 0.5       | Reviewing ZIP 317 for security and privacy.                                                       |
{{</table>}}

Total days: 17, Total paid: $17,000.

## November

In November, I audited zecwallet-lite-cli, the transaction processing library
used by ZecWallet-Lite. This paves the way for a future audit of ZecWallet-Lite
itself. I also looked at the customizations that were made to the version of
lightwalletd that ZecWallet-Lite uses.

{{<table "table table-striped table-bordered">}}
| Days      | Description                                                                                       |
|-----------|---------------------------------------------------------------------------------------------------|
| 1         | Catching up on forums, planning, setting up Calendly for office hours.                            |
| 8         | Auditing zecwallet-lite-cli.                                                                      |
| 1         | Reviewing changes to lightwalletd in aditapk00/lightwalletd.                                      |
| 1         | Helping debug zcashd v5.3.0 build issues and getting the Arch Linux package maintainer to update the package.    |
| 0.5       | Writing [Scalable Private Money Needs Scalable Anonymous Messaging]({{<ref "scalable-private-money-needs-scalable-private-messaging.md">}}). |
{{</table>}}

Total days: 11.5, Total paid: $11,500.

## December

In December, I focused on planning for the new year. I quickly surveyed every
Zcash-related project that I could find and collected a list of past security
audits, research, and notable bugs. I published the first edition of the [Zcash Ecosystem Security Overview]({{<ref "overview.md">}}) and a tentative [roadmap for 2023]({{<ref "zecsec-roadmap-for-2023.md">}}).

{{<table "table table-striped table-bordered">}}
| Days      | Description                                                                                       |
|-----------|---------------------------------------------------------------------------------------------------|
| 8         | Surveying all Zcash-related projects, preparing the ecosystem security overview and roadmap.      |
| 2         | Checking the Ywallet bug fixes and publishing the Ywallet audit report.                           |
{{</table>}}

Total days: 10, Total paid: $10,000.

## Conclusion

In Q4 of 2022, I completed two major audits of Ywallet and zecwallet-lite-cli. I
also contributed to a number of other security efforts such as performing
shorter security reviews of various projects, helping to investigate bugs,
collecting past security research, and writing up my research into options for
privately scaling Zcash.

In total, I invoiced for 38.5 days and was paid $38,500 in ZEC.

If you have questions about the work that was done or have suggestions for
future priorities, you can either email me at zecsec@defuse.ca or ask on [the
grant's forum
thread](https://forum.zcashcommunity.com/t/zcash-ecosystem-security-lead/42090).