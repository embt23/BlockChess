//! One validator, running one height.
//!
//! No I/O and no clock. [`Node::start`] and [`Node::receive`] return the
//! messages this validator wants sent; the caller — a test, a simulated
//! network, or eventually a socket — decides what happens to them. That is
//! what makes a four-node consensus run reproducible from a seed.
//!
//! ## Round 0 is the whole protocol, when it works
//!
//! Propose, prevote, collect ⅔, precommit, collect ⅔, commit. Safe by
//! quorum intersection, and implemented here in full. Everything that
//! happens when a round *fails* routes through [`crate::locking`], which is
//! the `G0` hole — see that file for why the seam is there.

use crate::commit::Commit;
use crate::locking::{self, Lock, Proposal};
use crate::msg::{Committed, Msg};
use crate::tally::{Outcome, Tally};
use crate::validators::ValidatorSet;
use crate::vote::{Step, Vote};
use bc_block::Block;
use bc_hash::Hash;
use bc_sig::{SigningKey, VerifyingKey};
use std::collections::BTreeMap;

pub struct Node {
    key: SigningKey,
    set: ValidatorSet,
    height: u64,
    round: u32,
    /// The proposal seen in the current round.
    proposal: Option<(Block, Option<u32>)>,
    prevotes: BTreeMap<u32, Tally>,
    precommits: BTreeMap<u32, Tally>,
    lock: Option<Lock>,
    /// Set once. A validator that has committed a height is done with it.
    committed: Option<Committed>,
    /// Blocks this node knows the body of, so it can commit on a
    /// certificate that arrives before the proposal did.
    known: BTreeMap<Hash, Block>,
    prevoted: bool,
    precommitted: bool,
}

impl Node {
    pub fn new(key: SigningKey, set: ValidatorSet, height: u64) -> Node {
        Node {
            key,
            set,
            height,
            round: 0,
            proposal: None,
            prevotes: BTreeMap::new(),
            precommits: BTreeMap::new(),
            lock: None,
            committed: None,
            known: BTreeMap::new(),
            prevoted: false,
            precommitted: false,
        }
    }

    pub fn id(&self) -> VerifyingKey {
        self.key.verifying_key()
    }

    pub fn round(&self) -> u32 {
        self.round
    }

    pub fn lock(&self) -> Option<Lock> {
        self.lock
    }

    pub fn committed(&self) -> Option<&Committed> {
        self.committed.as_ref()
    }

    pub fn is_proposer(&self) -> bool {
        self.set.proposer(self.height, self.round) == Some(self.id())
    }

    /// Equivocations this node has witnessed. Evidence for a slash.
    pub fn equivocations(&self) -> usize {
        self.prevotes
            .values()
            .chain(self.precommits.values())
            .map(|t| t.equivocations().len())
            .sum()
    }

    /// Enter the current round. If this node is the proposer, it proposes.
    ///
    /// `candidate` is the block it would build if it is free to choose. A
    /// locked proposer re-proposes its locked value instead, which is not
    /// part of the `G0` hole — it is the one piece of lock handling that is
    /// unambiguous and it belongs with the proposer logic.
    pub fn start(&mut self, candidate: Block) -> Vec<Msg> {
        self.known.insert(candidate.hash(), candidate.clone());
        if !self.is_proposer() {
            return Vec::new();
        }
        let (block, valid_round) = match self.lock {
            Some(l) => match self.known.get(&l.block) {
                Some(b) => (b.clone(), Some(l.round)),
                // Locked on a block whose body is gone. Cannot re-propose
                // what it cannot send, so it proposes nothing and the round
                // will time out — correct, and the reason liveness needs
                // more than one round.
                None => return Vec::new(),
            },
            None => (candidate, None),
        };
        vec![Msg::Propose {
            height: self.height,
            round: self.round,
            block,
            valid_round,
        }]
    }

    pub fn receive(&mut self, msg: Msg) -> Vec<Msg> {
        if self.committed.is_some() {
            return Vec::new();
        }
        match msg {
            Msg::Propose {
                height,
                round,
                block,
                valid_round,
            } => self.on_proposal(height, round, block, valid_round),
            Msg::Vote(v) => self.on_vote(v),
        }
    }

    fn on_proposal(
        &mut self,
        height: u64,
        round: u32,
        block: Block,
        valid_round: Option<u32>,
    ) -> Vec<Msg> {
        if height != self.height || round != self.round {
            return Vec::new();
        }
        if !block.body_matches_header() {
            return Vec::new();
        }
        if self.set.proposer(height, round) != Some(self.proposer_of(&block)) {
            // Only the round's proposer may propose in it.
            return Vec::new();
        }
        self.known.insert(block.hash(), block.clone());
        self.proposal = Some((block, valid_round));
        self.cast_prevote()
    }

    fn proposer_of(&self, block: &Block) -> VerifyingKey {
        VerifyingKey(block.header.proposer)
    }

    /// The one branch that touches the `G0` hole.
    fn cast_prevote(&mut self) -> Vec<Msg> {
        // Steps only go forwards. A node can hear a prevote quorum from
        // everyone else *before* the proposal reaches it — the network
        // reorders — and by then it has already precommitted and the
        // prevote step is over. Casting one now would be a vote in a step
        // this node has left, and it is also how a round-0 node acquires a
        // lock before prevoting, which is a state the protocol does not
        // have.
        if self.prevoted || self.precommitted {
            return Vec::new();
        }
        self.prevoted = true;
        let proposed = self.proposal.as_ref().map(|(b, vr)| Proposal {
            block: b.hash(),
            valid_round: *vr,
        });

        let value = match (self.round, self.lock) {
            // Round 0 with nothing to remember: prevote what was proposed,
            // nil if nothing was. No locking rule can apply, because there
            // is no earlier round to have been bound by.
            (0, None) => proposed.map(|p| p.block),
            // Anything else is a round that has to survive an earlier one.
            _ => locking::prevote_for(self.lock, proposed, self.round),
        };
        vec![Msg::Vote(Vote::sign(
            &self.key,
            self.height,
            self.round,
            Step::Prevote,
            value,
        ))]
    }

    fn on_vote(&mut self, v: Vote) -> Vec<Msg> {
        if v.height != self.height {
            return Vec::new();
        }
        let table = match v.step {
            Step::Prevote => &mut self.prevotes,
            Step::Precommit => &mut self.precommits,
        };
        let round = v.round;
        table.entry(round).or_default().add(&self.set, v);

        if round != self.round {
            return Vec::new();
        }
        match v.step {
            Step::Prevote => self.on_prevote_quorum(),
            Step::Precommit => self.on_precommit_quorum(),
        }
    }

    fn on_prevote_quorum(&mut self) -> Vec<Msg> {
        if self.precommitted {
            return Vec::new();
        }
        let value = match self.prevotes[&self.round].outcome(&self.set) {
            Outcome::Undecided => return Vec::new(),
            Outcome::QuorumNil => None,
            Outcome::Quorum(b) => Some(b),
        };
        self.precommitted = true;
        // Lock before precommitting, never after: the precommit is the
        // promise and the lock is what keeps it.
        self.lock = match (self.round, self.lock, value) {
            // First lock in round 0 on a real block: unambiguous.
            (0, None, Some(b)) => Some(Lock { round: 0, block: b }),
            (0, None, None) => None,
            _ => locking::lock_on_quorum(self.lock, value, self.round),
        };
        vec![Msg::Vote(Vote::sign(
            &self.key,
            self.height,
            self.round,
            Step::Precommit,
            value,
        ))]
    }

    fn on_precommit_quorum(&mut self) -> Vec<Msg> {
        let Outcome::Quorum(hash) = self.precommits[&self.round].outcome(&self.set) else {
            return Vec::new();
        };
        let Some(block) = self.known.get(&hash).cloned() else {
            return Vec::new();
        };
        let commit = Commit {
            round: self.round,
            votes: self.precommits[&self.round].votes_for(Some(hash)),
        };
        self.committed = Some(Committed { block, commit });
        Vec::new()
    }

    /// Everything this node has already said in the current round.
    ///
    /// Sent again after a reconnect. This is not a special case for the
    /// tests: a vote is a signed statement, repeating one costs nothing
    /// (`Tally::add` counts an identical repeat once), and a node that
    /// never re-sends can leave a healed network permanently one vote
    /// short of a quorum nobody can reassemble.
    pub fn resend(&self) -> Vec<Msg> {
        let me = self.id();
        self.prevotes
            .get(&self.round)
            .into_iter()
            .chain(self.precommits.get(&self.round))
            .filter_map(|t| t.vote_of(&me))
            .map(Msg::Vote)
            .collect()
    }

    /// The round timed out. Advance and try again.
    ///
    /// Everything reached from here is in the `G0` hole, because everything
    /// reached from here exists only because a round failed.
    pub fn timeout(&mut self, candidate: Block) -> Vec<Msg> {
        if self.committed.is_some() {
            return Vec::new();
        }
        self.round += 1;
        self.proposal = None;
        self.prevoted = false;
        self.precommitted = false;
        let mut out = self.start(candidate);
        // A proposer prevotes its own proposal; a non-proposer that has
        // seen none prevotes nil once the round opens.
        if out.is_empty() {
            out = self.cast_prevote();
        }
        out
    }
}
