---
title: "Free2Z Security Audit Results"
date: 2023-09-14T01:00:00-07:00
author: "Taylor Hornby"
draft: false
---

Below you will find a link to the results of my audit of
[Free2Z](https://free2z.com/), which occurred in February, 2023.

The audit found one cross-site scripting bug which could have been used to run
arbitrary JavaScript code in users' browsers. I also noted a privacy risk, which
is that Free2Z allows embedding content from third-party domains, which an
attacker could potentially use to track visits to their pages on Free2Z. Some
bugs were found in the implementation of Free2Z's "tuzi" token, which could have
been used to get tuzi tokens for free.

The cross-site scripting bug has been fixed, the potential privacy leakage has
been documented, and the tuzi token issues were fixed. All other bugs found were
minor.

[**Free2Z Audit Report (PDF)**](/audits/Free2Z%20Mini%20Audit-Final.pdf)