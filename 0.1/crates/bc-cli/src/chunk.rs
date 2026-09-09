//! `blockchess chunks` — experiment X9, the spatial version.
//!
//! `papers/09-lineage.md` §5 records that our grammar induces over *move
//! sequences* while Chase & Simon's chunks are *spatial configurations*, and
//! that the psychology claim should not be made until the spatial version
//! exists. This is it.
//!
//! Nothing here is told what castling is, that g1 is where a king goes, or
//! that pawns shelter kings. It counts how often (piece, square) facts occur
//! together against how often they would if they were independent.

use bc_chess::Position;
use bc_index::chunks::{fact_name, Counts};

use crate::board::draw;
use crate::study::load;

pub fn run(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut limit = usize::MAX;
    let mut from_ply = 20usize;
    let mut sample = 20_000usize;
    let mut want = 12usize;
    let mut max_size = 6usize;
    let mut i = 0;
    while i < args.len() {
        let n = |v: Option<&String>, f: &str| -> Result<usize, String> {
            v.ok_or(format!("{f} needs a number"))?
                .parse()
                .map_err(|_| format!("{f} needs a number"))
        };
        match args[i].as_str() {
            "--limit" => {
                i += 1;
                limit = n(args.get(i), "--limit")?;
            }
            "--from-ply" => {
                i += 1;
                from_ply = n(args.get(i), "--from-ply")?;
            }
            "--sample" => {
                i += 1;
                sample = n(args.get(i), "--sample")?;
            }
            "--show" => {
                i += 1;
                want = n(args.get(i), "--show")?;
            }
            "--max-size" => {
                i += 1;
                max_size = n(args.get(i), "--max-size")?;
            }
            other => path = Some(other.to_string()),
        }
        i += 1;
    }
    let path = path.ok_or("usage: blockchess chunks <file.pgn> [--from-ply N] [--sample N]")?;

    let games = load(&path, limit)?;

    // Sampling starts past the opening on purpose. The initial position is the
    // most frequent configuration in any corpus and is not an idea; letting it
    // in would drown everything else in "the pieces are on their home squares".
    let mut counts = Counts::default();
    let mut kept = 0usize;
    'outer: for g in &games {
        let mut pos = g.start;
        for (ply, &m) in g.moves.iter().enumerate() {
            if ply >= from_ply {
                let keep = kept < sample;
                counts.observe(&pos, keep);
                if keep {
                    kept += 1;
                }
                if kept >= sample && counts.positions > (sample as u64) * 3 {
                    break 'outer;
                }
            }
            pos = pos.make_move(m);
        }
    }

    if counts.sample.len() < 200 {
        return Err(format!(
            "only {} positions past ply {from_ply}; need a bigger corpus or a lower --from-ply",
            counts.sample.len()
        ));
    }

    // 1% of the sample. Below this a "chunk" is a handful of games, and lift
    // computed on a handful of games is noise with a big number attached.
    let min_support = (counts.sample.len() as u64 / 100).max(20);

    println!();
    println!("  CORPUS  {path}");
    println!("  {:<30}{:>10}", "games", games.len());
    println!("  {:<30}{:>10}", "positions counted", counts.positions);
    println!("  {:<30}{:>10}", "positions retained", counts.sample.len());
    println!("  {:<30}{:>10}", "sampled from ply", from_ply);
    println!("  {:<30}{:>10}", "min support", min_support);
    println!("  {:<30}{:>10}", "distinct pairs seen", counts.pair.len());

    print!("\n  searching… ");
    let t = std::time::Instant::now();
    let chunks = bc_index::chunks::discover(&counts, min_support, max_size, want);
    println!("{:.2}s", t.elapsed().as_secs_f64());

    if chunks.is_empty() {
        println!("\n  Nothing cleared the thresholds. Try --from-ply lower or a bigger corpus.");
        return Ok(());
    }

    println!();
    println!("  CHUNKS   groups of squares that occur together far more often");
    println!("  than independent pieces landing that way would explain.");
    println!();
    println!("  Lift 1.0 is exactly chance. Lower-case names with a leading");
    println!("  dot are Black's pieces.");

    for (n, c) in chunks.iter().enumerate() {
        println!();
        println!(
            "  #{:<3} {} facts   lift {:.0}×   in {} of {} positions ({:.1}%)",
            n + 1,
            c.facts.len(),
            c.lift(),
            c.support,
            counts.sample.len(),
            100.0 * c.support as f64 / counts.sample.len() as f64
        );
        println!(
            "       {}",
            c.facts
                .iter()
                .map(|&f| fact_name(f))
                .collect::<Vec<_>>()
                .join("  ")
        );
        // Draw the chunk alone on an empty board — the configuration is the
        // claim, and everything else in the position is noise around it.
        let mut board = Position::empty();
        for &f in &c.facts {
            let (col, piece, sq) = bc_index::chunks::unfact(f);
            board.color_bb[col.idx()] |= 1u64 << sq;
            board.piece_bb[piece.idx()] |= 1u64 << sq;
        }
        println!();
        print!("{}", draw(&board, None));
    }

    println!();
    if counts.sample.len() < 5_000 {
        println!(
            "  *** {} positions is too few, and lift is the wrong statistic to",
            counts.sample.len()
        );
        println!("  *** read on a small or homogeneous sample: the independence");
        println!("  *** baseline collapses and the ratio explodes. A six-figure lift");
        println!("  *** here means the sample is degenerate, not that the finding is");
        println!("  *** strong. Support is the number to trust until this passes a few");
        println!("  *** tens of thousands.");
        println!();
    }
    println!("  WHAT THIS IS AND IS NOT");
    println!("  Nothing above was told what castling is, or that g1 is where a king");
    println!("  goes. It counted co-occurrences of anonymous (piece, square) facts.");
    println!();
    println!("  But frequency is not meaning. A high-lift group can be a consequence");
    println!("  of the rules rather than an idea a player holds — pieces that never");
    println!("  moved sit together because neither moved. And these are rigid: exact");
    println!("  squares, no variable slots, so per papers/09-lineage.md §5 this finds");
    println!("  fixed configurations and misses the flexible schemas that Gobet &");
    println!("  Simon argue real expertise is made of.");
    println!();
    Ok(())
}
