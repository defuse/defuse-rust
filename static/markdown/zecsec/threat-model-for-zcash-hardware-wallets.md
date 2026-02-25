---
title: "A Simple Threat Model for Zcash Shielded Hardware Wallets"
date: 2023-04-15T00:00:00-07:00
author: "Taylor Hornby"
---

In this post, we'll share a simple threat model for Zcash shielded hardware
wallets. This is useful for informing users about which security and privacy
properties they can expect to rely on when using a shielded hardware wallet and
to help security auditors understand what counts as a "security bug."

To define a threat model for Zcash shielded hardware wallets, we consider the
following scenario:

1. The user initially sets up their hardware wallet using a PC or smartphone
which is fully compromised by the attacker. The user is careful to back up their
seed phrase and never enter it into the compromised PC/phone.
2. The user sends funds to their hardware wallet, using the addresses displayed
on its dedicated screen.
3. The user spends funds, carefully checking the transaction details displayed
on the dedicated screen are correct, and only approving the transaction on the
device when all details are as expected.

Within this scenario, we define the threat model in terms of what expectations
the user has regarding the attacker who has compromised their PC/phone,
following the process of [Invariant-Centric Threat
Modelling](https://github.com/defuse/ictm).

We expect that the attacker *CAN*:

1. Prevent the user from using their wallet on that compromised PC/phone (but not
others).
2. Steal viewing keys which let the attacker permanently compromise the privacy
of the wallet's addresses.
3. Cause funds being spent by the user to be stolen, if the user did not
carefully verify the transaction details on the device's screen.
4. Cause funds that the user is receiving to be stolen, if they did not verify
their address on the device's screen or if they used the compromised PC/phone to
transmit their address (e.g. the attacker could replace the user's address with
their own after the user pastes it into an exchange's website).
5. Cause transactions to be generated in a way that weakens privacy (e.g.  by
causing randomized signatures to not be randomized, or by tagging transactions
such that the sender's identity is revealed on the public blockchain).

The user expects that the attacker *CANNOT*:

1. Obtain the spend authority keys for any of the user's addresses.
2. Cause the user's funds to become unspendable.
3. Spend the user's funds without physical approval on the hardware device.
4. Produce a transaction whose semantics are different in any way from what was approved through the device's display, including changing the destination of funds being spent, the amount of funds being spent, or the fee amount.
5. Cause the hardware wallet to display, as its own address, any other address, such as one an attacker controls.
6. Cause an official-looking message to be displayed on the hardware wallet's display (which could be used for phishing).
7. Permanently prevent the hardware wallet from functioning with another uncompromised PC/phone.

In addition to these specialized considerations for shielded hardware wallets, a
PC or smartphone wallet app that interfaces with a shielded hardware wallet is
expected to fall under the scope of the [Zcash Wallet App Threat
Model](https://zcash.readthedocs.io/en/latest/rtd_pages/wallet_threat_model.html).
