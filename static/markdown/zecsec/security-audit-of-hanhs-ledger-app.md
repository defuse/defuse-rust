---
title: "Results of Auditing Hanh's Shielded Zcash Ledger App"
date: 2023-09-21T01:01:00-07:00
author: "Taylor Hornby"
draft: false
---

In May and June of this year, I reviewed Hanh Huynh Huu's Ledger app supporting
shielded Zcash. The full audit report is available at the link below.

## Summary of Results

The main issues uncovered by the audit were the following:

### Side-Channel Leakage

The algorithm for producing transaction signatures is not constant-time,
therefore it may be possible to leak the user's signing keys through a
side-channel attack. This will be fixed by making the code constant-time.

### Incompatibility with ZIP 32

There is an incompatibility between [ZIP 32](https://zips.z.cash/zip-0032)
and the key-derivation API currently provided on Ledger devices. 

ZIP 32 derives a chain of keys from the user's root seed. Ledger's API does not
support ZIP 32 key derivation, and there is no way for an app to access the root
seed to do the derivation itself, since allowing an app to access the seed would
be a security risk. The Ledger app therefore needed to generate its keys in a
way that is not compatible with ZIP 32. 

Given the current implementations of standard Zcash wallets and the Ledger app,
it would not be possible to import a standard wallet's seed phrase into a Ledger
and access shielded funds through the app. Vice-versa, importing a Ledger seed
phrase into a standard Zcash wallet would require the wallet to perform twice
the scanning effort to find all of the user's shielded funds.

The most-compatible way to resolve this issue would be for Ledger to add support
for ZIP 32 to their devices' operating systems. 

Failing that, ZIP 32 could be amended to be compatible with hardware wallets'
existing APIs; this would make seed phrases compatible between standard wallets
and the Ledger app, at the cost of needing to do twice the scanning effort when
importing old wallets. 

Another potential solution is to develop a tool for deriving individual shielded
signing keys from a Ledger seed phrase, so that they can be manually imported
into a standard shielded wallet. Unfortunately this approach would still not
allow a standard seed phrase with shielded funds to be imported into a Ledger.

### Burning Funds via Invalid Ciphertext

When a user's wallet receives a shielded Zcash transaction, it needs to receive
some additional information in order to spend the funds. This information is
put, by the sender, into the *note ciphertext*, which only the recipient's
wallet can decrypt.

A hardware wallet must ensure that these ciphertexts are produced honestly,
otherwise malware on the host computer could "burn" funds being sent by
replacing the ciphertext with junk data, preventing the recipient from accessing
the information needed to spend their funds.

The Ledger app is missing this ciphertext verification. It will be fixed by
having the hardware wallet compute (or re-compute and verify) the ciphertext,
rather than trusting the value provided by the host.

### Other

Several other minor issues were found, including an issue where unified
addresses were not correctly displayed to the user; see the audit report for
full details.

### Gaps in Coverage

While all of the ledger app's code was reviewed to some degree, there was not
enough time to thoroughly review some security-critical cryptographic code, like
the elliptic curve implementations. More review is needed in those areas; this
is discussed further in the report.

After the audit, we found that an implementation of one of the curves was using
an incomplete addition formula, when it should have been using a complete
addition formula. This issue has been reported separately.

## The Audit Report

[**Shielded Zcash Ledger App Audit Report (PDF)**](/audits/zcash-ledger-audit-report-v2.pdf)