# 01 — Cryptographic primitives

## Hash function

**`H = BLAKE3`**, 256-bit output.

Rationale: it is a Merkle tree internally, so hashing large structures
parallelises; it is fast in software without hardware acceleration; the
reference implementation is Rust.

For teaching purposes the first implementation should be **SHA-256**, because
its Merkle–Damgård construction (pad, chunk, compress, chain) is the clearest
possible illustration of "how do you hash something longer than one block", and
because you can implement it in an afternoon from the FIPS spec. Swap to BLAKE3
once the chain works.

### Domain separation

Every hash in the protocol is prefixed with a unique constant. Never hash raw
concatenations.

```
H_domain(D, x) = H( len(D) ‖ D ‖ x )       where D is an ASCII tag
```

Tags in use:

```
"BC/state/v1"       hash of a game state (the thing players sign)
"BC/pos/v1"         hash of a position   (used for repetition detection)
"BC/tx/v1"          hash of a transaction
"BC/header/v1"      hash of a block header
"BC/smt/leaf/v1"    sparse Merkle leaf
"BC/smt/node/v1"    sparse Merkle internal node
"BC/chanid/v1"      channel identifier derivation
"BC/resign/v1"      resignation message
"BC/draw/v1"        draw agreement message
"BC/vrf/v1"         matchmaking randomness
```

**Why this matters.** Without separation, a 64-byte structure signed in one
context can be reinterpreted as a different 64-byte structure in another. The
attack is: find two protocol messages whose byte encodings coincide, get a
signature on the harmless one, replay it as the dangerous one. The length prefix
also prevents `H("ab"‖"c") == H("a"‖"bc")` collisions from concatenation.

## Signatures

**`Ed25519`** (RFC 8032). 32-byte public keys, 64-byte signatures.

### Why Ed25519 and not ECDSA/secp256k1

1. **Deterministic nonces.** ECDSA needs a fresh random `k` per signature; reuse
   or bias in `k` leaks the private key algebraically, and this has destroyed
   real systems (PlayStation 3, several Bitcoin wallets). Ed25519 derives the
   nonce as `H(prefix ‖ message)`, so it cannot be gotten wrong by a bad RNG.
2. **Linearity.** Ed25519 is a Schnorr scheme: `s = r + H(R‖A‖m)·a`. The
   verification equation `[s]B = R + [H(R‖A‖m)]A` is *linear in the secret*.
   This is what makes key aggregation (MuSig2), threshold signatures, and
   adaptor signatures possible. ECDSA's inversion `s = k⁻¹(z + r·d)` destroys
   that structure. We will want aggregation later for server multisigs.
3. **Speed.** Batch verification of `n` signatures costs far less than `n`
   individual verifications — a real win for a validator verifying a block.

### The group

Ed25519 operates in the prime-order subgroup of the twisted Edwards curve

```
  −x² + y²  =  1 + d·x²·y²        over  F_p,  p = 2²⁵⁵ − 19
  d = −121665/121666
```

The curve has order `8ℓ` where

```
  ℓ = 2²⁵² + 27742317777372353535851937790883648493      (prime)
```

so the cofactor is 8. Secret scalars live in `Z/ℓZ`; public keys are points
`A = [a]B` for the standard base point `B`.

The Edwards addition law is **complete** — one formula works for every pair of
points including doublings and the identity — which is why Ed25519 has no
special-case branches and therefore no timing side channels from them. Compare
Weierstrass curves, where `P + P` and `P + Q` need different formulas.

The cofactor 8 is the source of Ed25519's one real subtlety: *malleability*. A
signature can sometimes be modified into another valid signature for the same
message. This does **not** break authenticity, but it breaks any assumption that
"signature bytes uniquely identify a signed message". **Rule: never use a
signature as an identifier or a map key.** Identify by the hash of the signed
content.

## Encoding

Canonical, deterministic, length-prefixed. Two rules:

1. **Fixed-width little-endian integers.** `u16`, `u32`, `u64`, `u128`.
2. **No optional fields, no maps, no floats.** Every structure has a fixed shape
   for a given version byte.

This gives **canonicality**: one value has exactly one byte encoding, so
`H(decode(encode(x))) == H(x)` always. Non-canonical encodings are the standard
source of consensus splits — two nodes that disagree about the bytes disagree
about the hash and therefore about the chain.

In practice: Rust structs with a hand-written `fn encode(&self, out: &mut Vec<u8>)`.
Do not use `serde` for consensus-critical structures. Serde is for RPC.

## Transport

**Noise protocol framework, `Noise_XX_25519_ChaChaPoly_BLAKE2s`.**

- `XX` — both parties transmit their static keys during the handshake, each
  authenticated. Neither needs to know the other's key in advance, which is what
  we want for matchmaking between strangers.
- Reuses the same curve as our signatures (X25519 and Ed25519 are the same curve
  in different coordinates), so one keypair type across the project.
- Gives forward secrecy: compromising a long-term key later does not decrypt
  recorded games.

The transport hides move content and timing from the network. It does **not**
hide *that* two IPs are talking — see `08-privacy.md`.

## Randomness

- Key generation: OS CSPRNG (`getrandom`).
- Matchmaking and pairings: **VRF** (RFC 9381, Ed25519-based), never raw RNG.
  A VRF output is verifiable — the server proves the pairing was determined by a
  committed seed and could not have been chosen. See `07-servers.md`.
- Nonces in signatures: derived, not sampled (see above).

## What we deliberately do not use yet

- **Pairings / BLS.** Needed only for signature aggregation across many
  validators and for SNARKs. Deferred to `08-privacy.md` and beyond.
- **ZK proof systems.** The v1 adjudicator reveals positions in plaintext during
  disputes. ZK settlement is the endgame, not the start.
- **Threshold signatures.** Only needed if servers become multi-operator.
