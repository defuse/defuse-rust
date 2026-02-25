---
title: "Risk Analysis of Intel's SGX and Other TEEs"
date: 2023-03-10T01:00:00-07:00
author: "Taylor Hornby"
draft: false
---

Intel's [Software Guard eXtensions (SGX)](https://www.intel.com/content/www/us/en/developer/tools/software-guard-extensions/overview.html) is a Trusted Execution Environment (TEE) technology built into some Intel CPUs. It is a special mode of operation that the processor can be put into that intends to allow for integrity-protected and confidentiality-protected processing, even under the assumption that the machine is fully compromised (i.e. the attacker has root access). 

Using cryptographic keys baked into the sillicon, SGX intends to allow the processor to remotely "attest" to its secure execution state, so that clients can be assured that they are interacting with safe code (the "enclave") and can provide secrets to that code without them being vulnerable to theft even by root users of the system.

In recent years, researchers have produced attacks that result in a complete breakage of SGX's security guarantees. The variety of methods used in the attacks and the frequency with which new bugs are being discovered in Intel processors calls into question SGX's ability to provide a secure Trusted Execution Environment. 

*What are these attacks, and are there any alternative TEE designs that fare better?*

## Recent History of Attacks on SGX


Prior to 2018, attacks on SGX enclaves had all relied on vulnerabilities in specific software written for SGX and did not break SGX itself. Attackers either found remote-code-execution type bugs in specific enclaves or exploited side-channel leakage in enclave code that was not carefully written to not leak data through timing and memory access patterns.

In 2018, this changed when the [Foreshadow attack](https://foreshadowattack.eu/) broke SGX to the point where attackers could forge remote attestations, violating both the integrity and confidentiality of any computations running inside SGX. 

Foreshadow is aptly-named; it preceeded a series of attacks which all broke SGX in similarly-bad ways. All of the following bugs have been mitigated with hardware patches and/or microcode updates; the goal of the survey below is to give a feel for the state-of-the-art in SGX attack research to inform your risk assessment when considering using SGX in your application.

### Foreshadow --- 2018

The [Foreshadow](https://foreshadowattack.eu/) attack exploits a transient execution bug in Intel processors to read an SGX enclave's protected memory. 

The way this is done is that the attacker runs a piece of code that attempts to read memory within the region allocated to the enclave. Normally, these reads should return -1 values so that the attacker cannot access the enclave's secrets. However, if (a) the attacker sets up memory access permissions  such that the access produces an exception rather than a -1 value and (b) the value at the memory location is in the processor's L1 cache, a race condition causes the processor to *transiently* execute the attacker's code on the actual value residing in the enclave's memory.

*Transient execution* is a performance optimization feature whereby the processor optimistically continues running code, even if that code would generate an exception (e.g. it gets run in parallel with a memory access-control check that fails) and even if the code would ordinarly not be executed at all (e.g. the processor mis-predicts which branch will be taken).

Most of the effects of the transiently-executed instructions are rolled back. However, the attacker, who controls the transiently-executed code, has enough time to encode a value they want to leak into the state of the processor's cache by reading a memory location that depends on the value. Their transiently-executed read puts the secret-value-dependent location into the cache. After the roll-back, another attacking process reads the memory locations that might have been transiently accessed. The location that was put into the cache will be read quickly, whereas the locations corresponding to the other possible values will be slow. As a result, the attacker learns the value that the transiently-executed code briefly had access to.

By combining a transient execution attack with some other tricks, Foreshadow's authors were able to leak the secrets from Intel's quoting enclave, an enclave that is instrumental in the remote attestation process. The leaked secrets allowed for forgery of remote attestations, meaning they could obtain any of the secrets clients would send into any enclave and arbitrarily corrupt the integrity of any enclave's execution.

### Plundervolt --- 2019

In [Plundervolt](https://plundervolt.com/), researchers exploited a software interface for controlling the processor's voltage and frequency to inject faults into SGX computations. By lowering the voltage, they found that they could cause long-lived instructions like multiplications and AES round computations to be computed incorrectly.

They demonstrated how to use these faults to (a) recover an RSA secret key for a certain implementation of RSA, (b) recover an AES key when the encryption is performed using Intel's hardware-accelerated AES-NI instruction set, and \(c\) create memory-corruption type bugs in otherwise bug-free enclaves by glitching the multiply instructions in size calculations so that wrong amounts of memory were allocated. They did not break the remote attestation process, but since remote attestation relies on similar cryptography, it's plausible that a well-engineered Plundervolt attack could have done so.

### SGAxe --- 2020

[SGAxe](https://sgaxe.com/files/SGAxe.pdf) uses [CacheOut](https://cacheoutattack.com/), yet another transient execution attack, to leak the keys needed to forge SGX attestations.

The CacheOut bug in particular highlights the nacency of our collective understanding of, and ability to fix, processor bugs. Prior to CacheOut's discovery, the previously-discovered transient execution bugs [Spectre and Meltdown](https://meltdownattack.com/) had already been patched by Intel. Despite the class of transient execution bugs being known for some time prior, the researchers behind CacheOut found that the patching was incomplete and that transient execution bugs still existed.

Interestingly, CacheOut leaks data from an undocumented data path in the processor. The researchers were able to leak the data---experimentally---without being aware of the true reason for the leakage. This is an example of how Intel's secrecy regarding its processor designs can be a hinderance to security research yet does not prevent the discovery of new attacks.


### ÆPIC Leak --- 2022

[ÆPIC Leak](https://aepicleak.com/) exploits a bug in the memory-mapping of [APIC](https://en.wikipedia.org/wiki/Advanced_Programmable_Interrupt_Controller) registers. 

The APIC's registers are mapped into normal memory space so that the operating system's kernel can read and modify them. However, the registers are split into 4-byte values and those values are aligned along 64-byte boundaries. In-between the values are regions that software is never meant to read or write to. Technically, it's "undefined behaviour" if software ever does read or write there.

What the researchers found was that actually, reading from those regions would sometimes return whatever data happened to be in the processor's L1 cache. They combined this bug with some other tricks to get regions of SGX enclaves' memory into the L1 cache, allowing them to dump any enclave's memory and registers. By attacking Intel's quoting enclave, they were able to get the keys needed to forge remote attestations, breaking the confidentiality and integrity of all enclaves.

## Other TEEs, Alternatives to SGX

### AMD's SEV

AMD's alternative to Intel's SGX is its Secure Encrypted Virtualization (SEV), which aims to encrypt virtual machines' memory so that their memory contents are not accessible to the hypervisor they are running on.

AMD's processors were vulnerable to many of the same kinds of transient execution attacks as Intel's processors were (e.g. Spectre), so SEV is likely vulnerable to similar kinds of attacks. Not as much security research exists on AMD's SEV as does for Intel's SGX, nevertheless attack research has shown that its remote [attestation can be foiled](https://dl.acm.org/doi/10.1145/3319535.3354216) and [virtual machines can be impersonated](https://dl.acm.org/doi/10.1145/3460120.3485253), eroding the trust that remote clients might place in SEV-encrypted virtual machines.

### ARM's TrustZone

ARM CPUs include a similar technolgy called TrustZone which [has similarly been catastrophically broken by bugs in the past](https://ieeexplore.ieee.org/document/9152801). In contrast to SGX, the known attacks against TrustZone have mainly been remote-code-execution style attacks against the large corupus of software that runs inside and maintains TrustZone's TEE, as well as [side-channel attacks targeting cryptography](https://duo.com/decipher/new-side-channel-attack-extracts-private-keys-from-some-qualcomm-chips) running within TrustZone. That's not a guarantee that CPU bugs don't exist. In fact, voltage glitching can be used to break TrustZone's security, but these kinds of attacks have been [deemed to be out of scope](https://developer.arm.com/documentation/ka005159/1-0) as TrustZone only aims to defend against software-based attacks, not physical access attacks.


## Conclusion

Considering all of these attacks together, it is clear that TEE designs have not yet matured enough to be relied on for security-critical applications. Based on the rate at which bugs are being found, we can conclude that our scientific understanding of processor bugs is still in its infancy, as is our understanding of how to build bug-free TEEs. We should expect more bugs to be found in the coming years.

In the case of SGX, recent bugs have not only broken specific enclaves with poor side-channel defenses, they have catastrophically broken all of SGX's security guarantees, with demonstrations of forged remote attestations. Developers wishing to rely on SGX should design their applications under the assumption that SGX will become vulnerable to a publicized bug in the near future, potentially for as long as 6-12 months as microcode/hardware fixes are developed. It might also be wise to assume that nation-state actors are aware of undisclosed processor bugs in order to retain the capability of breaking SGX and other TEEs.

*Will it ever be safe to rely on TEEs?*

If highly-resourced attackers with physical access are in your threat model, I would say *no*. Since the processor must *somewhere* operate on the enclave's data, that data must physically reside within the processor, so it is vulnerable to theft by a sophisticated-enough attack; TEEs secure against highly-resourced attackers with physical access are fundamentally impossible[^1].

For less-critical applications, I would wait for the bug discovery rate to fall
much lower, say to one attestation-breaking bug discovered every three-or-so
years, before placing a significant amount of trust in any TEE. A low bug
discovery rate doesn't guarantee security, but you still can benefit from a TEE
when the cost of finding and exploiting a bug becomes greater than the value an
attacker would gain by breaking the TEE in your security model.

If you're only using a TEE as a defense-in-depth measure, there is no harm in doing so, just be sure to weigh the additional development costs against other ways you might invest in security.

[^1]: The exception to this is fully-homomorphic encryption (FHE), which uses cryptographic techniques, rather than trusted hardware, to ensure private execution. FHE is still impractical for most use cases.