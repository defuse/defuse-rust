---
title: "Security Audit Process"
date: 2022-12-29T07:00:00-07:00
author: "Taylor Hornby"
draft: false
---

In this post, I'm going to shed light on the process I follow to find bugs in my
audits of Zcash ecosystem software. Hopefully, this will be useful to you if you
are looking to understand more about how audits work or even learning how to
audit software yourself.

The process I'm describing here should not be taken as gospel.  Different
security auditors follow different processes, and I often deviate from this
process significantly to tailor my audits to the specific projects I am helping.

## Audit Process

### Scoping and Timeboxing

The first step in any audit is to understand *what you're auditing* and *how
long you have to audit it*. This is crucial, because auditor-time is an
exceptionally rare resource, so it needs to be applied effectively and to the
right thing.

In commercial engagements, the client and auditor often roughly agree on a
timeframe and budgeting (e.g. 2 weeks of auditing at $2,000 per auditor-day),
and have a back-and-forth discussion about the scope and sign mutual contracts
before the engagement begins. This is an inefficiency in the process that I
personally like to avoid, and thankfully past clients have trusted me to choose
my own scope, prioritizing what I feel is most important for the client. A
detailed scoping process makes sense whenever the client is already aware of
specific kinds of risks they need to mitigate, or if multiple auditors are being
hired to look at the same project and are to be tasked with different focus
areas.

Out of this step in the process, you want to know how many days of paid time you
have, what general categories of bugs you'll be looking for, and if anything is
off-limits (such as pen-testing production systems). You should also make it
clear up front whether or not the audit report will be published, as audit firms
are sometimes hesitant to do so, not wanting their report to be seen as a
"guarantee of security" or "stamp or approval", and clients may not want their
mistakes to be publicized.

### Brainstorming & Target Research

I see auditing as a creative process, so once any engagement begins, the first
thing I do is start brainstorming. Before I look at anything specific to the
target project, I think about what kind of project it is and try to enumerate
all the things that could possibly go wrong with *that kind of thing*. For
example, if it's an encryption program, I think about: weak key generation,
nonce reuse, weak encryption algorithms, lack of authentication, side-channel
attacks, and so on. I write all of these possibilities down. The point of this
exercise is to bring all of the things that *might* go wrong into your
short-term memory so that they will stand out to you, if they actually *have*
gone wrong, while you're reading the relevant code.

Next, I learn as much as I can about the target project without diving into
anything too technical. I read the docs, the changelog, issues that look
relevant to security on the issue tracker, google for past vulnerabilities, and
read any past audit reports of the project. I build and use the project, too. As
I am doing that, I keep expanding my brainstorm notes with new ideas that come to
mind.

At the end of the brainstorming and research phase, you should have a list of
*potential bugs* and *attacker goals*. Your potential-bugs list is a brainstorm
that ideally includes all weaknesses that could possibly exist in the target
project (e.g. weak random number generator), and your attacker-goals list is a
brainstorm that ideally includes any goals an attacker might have (e.g.  decrypt
the data). 

Your goal, during the audit, will be to find a set of weaknesses that an
attacker could exploit to accomplish their goal (e.g. the random number generator
produces weak keys, so an attacker can brute-force the key and decrypt the
data). 

### Threat Modeling

At this point, you have a good understanding of what the target project is, what
weaknesses it might have, and the various ways attackers might try to attack its
users. If time permits, you can write up a formal (or at least detailed) threat
model for the project. If you do this right, it will help the developers
understand the security properties of their own software that they need to
maintain in the future, help users understand what security properties they're
actually getting, and speed up future audits.

There are many valid approaches to threat modeling, a common one is
[STRIDE](https://en.wikipedia.org/wiki/STRIDE_(security)), however I prefer an
approach that I call [Invariant-Centric Threat
Modeling](https://github.com/defuse/ictm) which puts users first, requiring
security properties to be written in language that users can actually
understand, and considers it a "bug" if users ever think they are getting some
security property that they actually are not.

### Source Code Review

Finally, the actual review process begins. I start by understanding the
directory structure of the project (the ``tree`` command is very helpful!).
Then, I start reading code file-by-file. If there is an entrypoint to the
project (i.e. ``main())``), I will usually start there. But generally, I will
just read each file, in alphabetical order.

You might be a little surprised by the fact I read the files in alphabetical
order. Wouldn't it be better to *understand* the program by tracing through call
stacks, understanding how classes relate to each other by drawing UML diagrams,
and so forth?  In a time-crunched audit (as all audits are), this actually isn't
the best use of time. It's important to understand how the code you're reading
fits in to the overall structure of the program, but if the code is well-written
and classes and functions are well-named, you can often correctly guess how it
fits in. In practice, I find most of my bugs in this linear scan of the code. A
linear scan of the code also guarantees a kind of completeness: you can be sure
that you've at least had your eyeballs pointed at every single line of code in
the project.

For this to work, the brainstorming process mentioned above is essential.
Without it, problems won't "jump out" at you as you are reading through the
code. It's also important to *continue* that same brainstorming process *as* you
read the code, making note of any new ideas for potential problems that come to
mind as you are diving deeper into the code. Make notes, like "this function
returns -1 on error, if the caller doesn't check it, a decryption error might be
unnoticed, is the caller actually checking it?", instead of getting bogged down
confirming problems in this part of the process.

After the first pass of source code review, you should have an expanded set of
notes with some potential issues, which you'll want to check later. I use a star
system to help prioritize which possible problems are most-worth looking into
later, e.g. "*** it looks like it's using ``rand()`` to generate key!?", "* make
sure the caller doesn't pass a negative number here or else this will panic".

You should also be confused about a lot of things and have notes like "I don't
get how this works!?", "*** wtf how can this possibly be secure!!??". In this stage,
we've been going for breadth and speed rather than depth and confirmation; this
is normal.

This is a good point to pause and do even more brainstorming. Ask yourself "what
else could go wrong?", "could any of the more-minor problems I've
identified be combined together to attack the system in bigger ways?"

### Check Priority Potential Issues

After reviewing most or all of the code, you should have a much better
understanding of how the code works, where things are, and a big list of TODOs:
possible problems and a bunch of things you're really confused about. The next
step is to confirm some of those potential issues. This is the part where you
actually need to start tracing through call stacks, understanding classes'
relationships to each other, understanding if vulnerable-looking code is
actually something an attacker's input can reach, and so on.

If you're really time-crunched, you won't be able to check everything, so
prioritize by which problems you intuitively think are most likely to be "real",
and by which kinds of problems would be the worst for the project's users.

A key thing to look for in this stage is the correctness (or incorrectness) of
how different components of the software interface with each other. This is
especially true when the different components were written by different people,
e.g. if your target software uses a library, you want to carefully read that
library's documentation and make sure it's being used correctly.

At the end of this stage, you should have found the bulk of the issues which you
can begin writing up in your report.

### Deep Analysis for Correctness

Following the steps above will give you a broad and efficient coverage of the
entire project, and you will likely have found most of the bugs that will make
it into your report. For some kinds of code, it's unavoidably necessary to dive
in much deeper and check that *everything* is correct.

"Correctness" means that the code always does what it is intended to, i.e. that
there are no bugs at all. As much as security engineers like to talk about
"security bugs" as a distinct category from "ordinary" kinds of bugs,
security is really about correctness. That's because *any* kind of bug, even
ones that seem benign, can potentially be used by an attacker to further their
goals.

For certain kinds of code, especially cryptography, parsers, and anything
enforcing access control, there is no way to assure security except to deeply
understand and ensure the absolute correctness of all of the relevant code. The
modern way to do this is via formal verification, a computer-checkable proof
that the code will always do exactly what it is intended to do. Formal
verification is expensive to implement (but tools are getting better), and it
depends on having a detailed specification of what the code is "supposed to do",
so it is not always a viable option.

The next best thing is for a human---the auditor---to carefully check that
everything is correct. Based on the preceding stages, you should have a good
idea of which parts of the target program are ultra-security-critical and are
worthy of checking for correctness in extreme detail. The mindset to be in here
is that of someone trying to *prove* the code is correct. Unless you are
absolutely, mathematically, convinced that the code is correct and conforms to
its specification, you still have more work to do.

Doing this is extremely costly; whereas you might be able to get through a dozen
source code files in a day of normal review, it might take an entire day of
careful thinking to ensure the correctness of just one part of a cryptographic
algorithm or protocol.

There are some tricks to avoid this, such as writing randomized tests to compare
two implementations of cryptographic primitives against each other, but to
whatever degree the relevant code is absolutely essential for security, you
often have to spend a lot of time checking its correctness.

### The Writeup

At last, it's time to write up the report. The format I like to use is:

1. Introduction -- describe the audit scope and write up the threat model for the project (if they don't already have one).
2. Security & Privacy Findings -- list each issue that you've found, describing its severity and recommended prioritization, explaining it in detail, and recommend ways to fix it.
3. Recommendations -- make suggestions to improve the overall quality of the code and/or speed up future audits.
4. Good Things -- say genuine good things about the project/code, so that the report isn't all negative.
5. Future Work -- list the things you couldn't cover in this audit, but should be covered by future audits. This section will be invaluable to future auditors.
6. Conclusion -- summarize everything briefly.

### Remediation & Checking the Fixes

It is common for fixes to reported security bugs to be incomplete or incorrect,
so it's important to evaluate the fixes to the bugs you have reported. A really
common pattern is for developers to "block the exploit", i.e. prevent one way of
exploiting the issue you described, without fixing the actual underlying
problem. I like to remain available to the developers of the projects I audit
until I've confirmed that all of the issues are fixed or we've mutually agreed a
fix is unnecessary or is safe to de-prioritize.

## Tooling & Automation

What I've described above is for a manual security review. Sometimes it's
useful to use tools to assist with the audit process. However, I personally
haven't found much success using security-specific tooling in my audits.
Anything an XSS or SQL injection scanner might find would definitely be found in
the manual review I would be doing anyway.

There are some exceptions, where automation and tooling is really helpful:

### Unit, Integration, and Regression Tests

A unit test or integration test of a software's security property is worth a
thousand reviews, because it can be run automatically upon every change to the
software, perpetually ensuring that the property is satisfied.

It might feel stupid, but make sure all the basic tests are in place, like "try
to decrypt the file with the wrong password" or "make the software connect to a
server with a self-signed certificate." Even stupid tests will catch bugs
sometimes.

Think about all the edge cases the target software might encounter, and
recommend that all of those edge cases be tested.

Whenever a security bug is found, do the best you can to test for the presence
of that-or-similar bugs, so that if it ever gets re-introduced, it will be caught quickly.

### Dependency Update Checkers

Tools like ``cargo audit``, ``./gradlew dependencyCheckAnalyze``, ``npm
outdated`` are invaluable for finding dependencies that are out of date or have
known vulnerabilities. These kinds of tools are available for most languages and
platforms, use them!

### Fuzzing

Fuzzing is the process of using various types of algorithms to generate "random"
input to throw at your program to try to find bugs and crashes.
Counter-intuitively, fuzzing can be [incredibly successful at finding
bugs](https://lcamtuf.coredump.cx/afl/), especially in projects written in
unsafe languages like C/C++.

You can also "fuzz" implementations of cryptographic algorithms against each
other. If two implementations of a cipher or hash function, say, are supposed
to be equivalent, you can find differences between them by throwing the same
random data at both, as long as any "bug" occurs with a sufficiently high
probability.

## To Write Exploits, or Not?

So far, I've written about how to find bugs, but what about exploiting them?
Exploitation is often the funnest part of security work, but it often isn't
necessary. If the client understands the bug and is going to fix it, then it
usually isn't worth the time to develop an exploit. 

The two exceptions are: (a) if the client doesn't acknowledge the existence of a
bug and you need to prove it's real, you need to write an exploit, and (b) if
you've got a severe bug on your hands, and you need to understand its
consequences or exploitability more-precisely, it can be beneficial to write an
exploit.

I personally prefer to make false-positive errors in my security work than
false-negative errors. That is, if I'm unsure about the exploitability of a
given issue, I'll include it in my report anyway, accepting a small chance that
I'm wrong. This saves the cost of rigorously proving the existence of the
problem, and makes sure that nothing that could be a real issue gets dropped.
The downside is that you might be embarrassed by reporting a bug that isn't
real, but that's better than the alternative.