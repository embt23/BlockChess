//! What validators send each other, and what a node concludes.
//!
//! Two messages. That is the whole protocol on the wire: somebody proposes
//! a block, and everybody votes twice about it.

use crate::commit::Commit;
use crate::vote::Vote;
use bc_block::Block;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    Propose {
        height: u64,
        round: u32,
        block: Block,
        /// Tendermint's `validRound`: the round whose prevote quorum
        /// justifies re-proposing this block.
        valid_round: Option<u32>,
    },
    Vote(Vote),
}

/// What the node has concluded, if anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    pub block: Block,
    pub commit: Commit,
}
