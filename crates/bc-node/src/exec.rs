//! Executing the canonical chain against the escrow.

use crate::action::Action;
use bc_block::{Block, Consensus};
use bc_channel::ledger::Ledger;
use bc_hash::Hash;
use bc_sig::VerifyingKey;

/// Opening balances. The only thing that creates value (`E1`), and it
/// happens before block zero rather than in any block.
#[derive(Debug, Clone, Default)]
pub struct Seed {
    pub balances: Vec<(VerifyingKey, u128)>,
}

impl Seed {
    pub fn credit(mut self, who: VerifyingKey, amount: u128) -> Seed {
        self.balances.push((who, amount));
        self
    }

    fn fresh_ledger(&self) -> Ledger {
        let mut l = Ledger::new();
        for (who, amount) in &self.balances {
            l.credit(who, *amount);
        }
        l
    }
}

/// A validator or full node: a consensus engine, and the state its
/// canonical chain implies.
pub struct Node<C: Consensus> {
    pub consensus: C,
    seed: Seed,
    ledger: Ledger,
    /// Chain heads whose replay produced the current ledger, newest last.
    executed: Vec<Hash>,
}

impl<C: Consensus> Node<C> {
    pub fn new(consensus: C, seed: Seed) -> Node<C> {
        let ledger = seed.fresh_ledger();
        let mut n = Node {
            consensus,
            seed,
            ledger,
            executed: Vec::new(),
        };
        n.apply();
        n
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Blocks currently executed, oldest first. What the node believes
    /// happened.
    pub fn history(&self) -> &[Hash] {
        &self.executed
    }

    pub fn balance(&self, who: &VerifyingKey) -> u128 {
        self.ledger.balance(who)
    }

    /// Offer a block to consensus and, if the head moved, recompute the
    /// state from the new canonical chain.
    pub fn submit(&mut self, block: Block) -> Result<(), C::Error> {
        let before = self.consensus.head();
        self.consensus.submit(block)?;
        if self.consensus.head() != before {
            self.apply();
        }
        Ok(())
    }

    /// Rebuild the ledger from the canonical chain.
    ///
    /// No undo, no journal, no special case for a reorg. The state is a
    /// pure function of the canonical chain, and this is that function.
    pub fn apply(&mut self) {
        self.ledger = self.seed.fresh_ledger();
        self.executed.clear();

        let chain = self.canonical_blocks();
        for (height, block) in chain {
            self.ledger.height = height;
            self.executed.push(block.hash());
            for tx in &block.txs {
                let Some(action) = Action::decode(tx.payload, &tx.body) else {
                    // An undecodable body is a transaction this node
                    // cannot execute. It is skipped, not fatal: on a real
                    // chain that is an upgrade it has not taken.
                    continue;
                };
                self.execute(action, height);
            }
        }
        self.ledger.height = self.consensus.height();
    }

    fn execute(&mut self, action: Action, height: u64) {
        // Every arm ignores its error on purpose. A transaction that the
        // escrow rejects is a transaction that reverted — it cost its
        // sender gas and changed nothing, which is exactly what should
        // happen and is not the node's problem.
        match action {
            Action::OpenGame {
                offer,
                white_sig,
                black_sig,
            } => {
                let _ = self.ledger.open_game(&offer, &white_sig, &black_sig);
            }
            Action::DisputeOpen {
                initiator,
                signed,
                packed_pos,
            } => {
                let _ = self
                    .ledger
                    .dispute_open(initiator, &signed, &packed_pos, height);
            }
            Action::DisputeMove {
                channel_id,
                mover,
                mv,
                claim,
            } => {
                let _ = self
                    .ledger
                    .dispute_move(&channel_id, mover, mv, height, claim);
            }
            Action::DisputeFinalize { channel_id } => {
                let _ = self.ledger.dispute_finalize(&channel_id, height);
            }
            Action::Unsupported => {}
        }
    }

    /// The canonical chain as `(height, block)`, oldest first.
    fn canonical_blocks(&self) -> Vec<(u64, Block)> {
        let mut out = Vec::new();
        let mut hash = self.consensus.head();
        while let Some(b) = self.consensus.block(&hash).cloned() {
            let height = b.header.height;
            let parent = b.header.parent_hash;
            out.push((height, b));
            if height == 0 {
                break;
            }
            hash = parent;
        }
        out.reverse();
        out
    }
}
