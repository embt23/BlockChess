"""What fraction of a stored game is actually chess?

Compression of the move stream is only worth doing until the move stream stops
being the big term. This script finds where that happens. The cryptographic
envelope -- the keys and signatures that say who played and that they agree the
game happened -- does not compress at all, and it is sized in bytes while the
game is sized in bits.
"""

from math import log2

GAME_PLIES = 80

ED25519_PUBKEY = 32
ED25519_SIG = 64
HASH = 32

PAYLOADS = [
    ("PGN text",                    40.00 * GAME_PLIES / 8),
    ("16-bit moves",                16.00 * GAME_PLIES / 8),
    ("legal index, b = 30",        log2(30) * GAME_PLIES / 8),
    ("model-based, 1.7 bits/ply",    1.70 * GAME_PLIES / 8),
]

# Per game, if every game is independently attested by both players.
ENVELOPE = [
    ("two public keys", 2 * ED25519_PUBKEY),
    ("two signatures", 2 * ED25519_SIG),
    ("result, rules id, clock, date", 8),
]
ENVELOPE_TOTAL = sum(v for _, v in ENVELOPE)


def batched(n_games: int) -> float:
    """Envelope cost per game when n_games are attested as one batch: the batch
    carries one aggregate signature and one Merkle root, and each game still
    carries a compact reference to each player's identity."""
    per_batch = ED25519_SIG + HASH + 8
    per_game = 2 * 4 + 8          # two 32-bit account ids, plus the metadata
    return per_batch / n_games + per_game


if __name__ == "__main__":
    print("envelope, one attestation per game")
    for name, v in ENVELOPE:
        print(f"  {name:<32} {v:>5} bytes")
    print(f"  {'total':<32} {ENVELOPE_TOTAL:>5} bytes")
    print()

    print(f"{'move stream':<28} {'bytes':>8} {'+envelope':>10} {'chess %':>9}")
    for name, payload in PAYLOADS:
        total = payload + ENVELOPE_TOTAL
        print(f"{name:<28} {payload:>8.1f} {total:>10.1f} {100 * payload / total:>8.1f}%")
    print()

    print("envelope per game when games are attested in batches")
    print(f"{'batch size':>11} {'envelope B':>11} {'chess % at 49 B':>17}")
    for n in (1, 10, 100, 1000, 10000):
        env = batched(n) if n > 1 else ENVELOPE_TOTAL
        payload = log2(30) * GAME_PLIES / 8
        print(f"{n:>11} {env:>11.1f} {100 * payload / (payload + env):>16.1f}%")
    print()

    print("corpus size at 6e9 games")
    for name, payload in PAYLOADS:
        for label, env in (("per-game attestation", ENVELOPE_TOTAL),
                           ("batched by 1000", batched(1000))):
            gib = 6e9 * (payload + env) / 1024 ** 3
            print(f"  {name:<28} {label:<22} {gib:>8.1f} GiB")
