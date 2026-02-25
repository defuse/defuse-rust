---
title: "Security Audit of zecwallet-lite-cli"
date: 2023-09-23T01:01:00-07:00
author: "Taylor Hornby"
draft: false
---

In November of 2022, I audited 
[zecwallet-lite-cli](https://github.com/adityapk00/zecwallet-light-cli), which
is both a command-line Zcash wallet and the library backing [ZecWallet
Lite](https://zecwallet.co/).

It appears that zecwallet-lite-cli is no longer being actively maintained, 
therefore the security issues found by this audit *have not been fixed.* I
recommend that users of ZecWallet Lite switch to
[Zingo](https://www.zingolabs.org/), [Ywallet](https://ywallet.app/), or
[Nighthawk](https://nighthawkwallet.com/), which are actively maintained.

The most notable security issue was found was in zecwallet-lite-cli's
auto-shielding behavior. By default, the wallet will auto-shield any transparent
funds opportunistically each time the user sends a shielded transaction. This
means that if an attacker knows the user's transparent address, they can
continually send small amounts of ZEC there to maintain the user's transparent
balance, and then all of the user's shielded transactions will be deanonymized,
since they will include a component that spends the user's transparent funds.

Several other issues of low to moderate impact were also found. See the full
report for details.

[**Security Audit of zecwallet-lite-cli**](/audits/zecwallet-lite-cli-audit-report-v2.pdf)