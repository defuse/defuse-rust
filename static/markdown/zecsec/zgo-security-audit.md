---
title: "ZGo Security Audit Results"
date: 2024-01-05T00:00:00-07:00
author: "Taylor Hornby"
draft: false
---

In April of last year, I performed a security audit of [ZGo](https://zgo.cash/),
a payment processing platform built on Zcash. The report is linked below.

My review found several critical security vulnerabilities. For example, one
vulnerability allowed an attacker to intercept and steal Zcash funds as they
were being sent by users to pay for their ZGo orders. It was also possible to
mark ZGo orders as paid without actually paying, and there were several issues
that compromised user privacy.

The ZGo team quickly fixed the issues. At the present time, all significant
issues have been verified as fixed.

The audit report and vulnerabilities described therein should be of independent
interest to Zcash developers, since they highlight some (perhaps common)
difficulties of working with the `zcashd` API to build a payment processing
platform. By making API improvements, perhaps some of these issues could be
avoided.

[**Security and Privacy Analysis of ZGo (PDF)**](/audits/ZGo-Security-Audit-v1.1.pdf)
