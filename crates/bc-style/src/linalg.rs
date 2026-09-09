//! The linear algebra the compression needs, written from scratch.
//!
//! Only three things are required: standardise a data matrix, form its
//! covariance, and diagonalise that covariance. The third is the only one with
//! any depth, and it is done with **Jacobi rotations** — the slow, obviously
//! correct algorithm, chosen for the same reason `bc-chess` generates
//! pseudo-legal moves and filters them: the version you can check by eye is
//! worth more than the fast one you cannot.
//!
//! Jacobi applies a sequence of plane rotations, each of which zeroes one
//! off-diagonal pair and is exactly orthogonal by construction. The sum of
//! squares of the off-diagonal entries is non-increasing, so the iteration
//! cannot diverge, and `V` stays orthonormal to machine precision because it is
//! only ever multiplied by rotations. That is the whole argument.

/// A dense, row-major matrix. Sized for `d ≈ 20`, not for scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}

impl Matrix {
    pub fn zeros(rows: usize, cols: usize) -> Matrix {
        Matrix {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    pub fn identity(n: usize) -> Matrix {
        let mut m = Matrix::zeros(n, n);
        for i in 0..n {
            m.set(i, i, 1.0);
        }
        m
    }

    pub fn from_rows(rows: &[Vec<f64>]) -> Matrix {
        let r = rows.len();
        let c = if r == 0 { 0 } else { rows[0].len() };
        assert!(rows.iter().all(|x| x.len() == c), "ragged matrix");
        Matrix {
            rows: r,
            cols: c,
            data: rows.concat(),
        }
    }

    #[inline]
    pub fn get(&self, r: usize, c: usize) -> f64 {
        self.data[r * self.cols + c]
    }

    #[inline]
    pub fn set(&mut self, r: usize, c: usize, v: f64) {
        self.data[r * self.cols + c] = v;
    }

    pub fn row(&self, r: usize) -> &[f64] {
        &self.data[r * self.cols..(r + 1) * self.cols]
    }

    /// The `c`-th column, copied out.
    pub fn col(&self, c: usize) -> Vec<f64> {
        (0..self.rows).map(|r| self.get(r, c)).collect()
    }
}

/// Per-column mean and standard deviation (population, `/n`).
///
/// A column with zero variance carries no information about any player, so its
/// deviation is reported as zero and [`standardise`] maps it to zero rather
/// than dividing by it.
pub fn column_stats(m: &Matrix) -> (Vec<f64>, Vec<f64>) {
    let n = m.rows as f64;
    let mut mean = vec![0.0; m.cols];
    let mut dev = vec![0.0; m.cols];
    if m.rows == 0 {
        return (mean, dev);
    }
    for r in 0..m.rows {
        for (c, mu) in mean.iter_mut().enumerate() {
            *mu += m.get(r, c);
        }
    }
    for mu in mean.iter_mut() {
        *mu /= n;
    }
    for r in 0..m.rows {
        for (c, d) in dev.iter_mut().enumerate() {
            let x = m.get(r, c) - mean[c];
            *d += x * x;
        }
    }
    for d in dev.iter_mut() {
        *d = (*d / n).sqrt();
        if *d < 1e-12 {
            *d = 0.0;
        }
    }
    (mean, dev)
}

/// Z-score every column against supplied statistics.
///
/// The statistics are taken as arguments rather than recomputed because they
/// are a property of the **corpus**: projecting a new game under an existing
/// lens must use the lens's own `μ` and `σ`, never the new game's.
pub fn standardise(m: &Matrix, mean: &[f64], dev: &[f64]) -> Matrix {
    let mut z = Matrix::zeros(m.rows, m.cols);
    for r in 0..m.rows {
        for c in 0..m.cols {
            let v = if dev[c] == 0.0 {
                0.0
            } else {
                (m.get(r, c) - mean[c]) / dev[c]
            };
            z.set(r, c, v);
        }
    }
    z
}

/// `Zᵀ Z / (n − 1)`, the sample covariance of an already-centred matrix.
pub fn covariance(z: &Matrix) -> Matrix {
    let d = z.cols;
    let denom = if z.rows > 1 { (z.rows - 1) as f64 } else { 1.0 };
    let mut c = Matrix::zeros(d, d);
    for i in 0..d {
        for j in i..d {
            let mut s = 0.0;
            for r in 0..z.rows {
                s += z.get(r, i) * z.get(r, j);
            }
            let v = s / denom;
            c.set(i, j, v);
            c.set(j, i, v);
        }
    }
    c
}

/// Eigenvalues and eigenvectors of a symmetric matrix, by Jacobi rotation.
///
/// Returns `(values, vectors)` sorted by descending eigenvalue, where column
/// `k` of `vectors` is the unit eigenvector for `values[k]`.
///
/// Each eigenvector is then **sign-canonicalised**: flipped so its
/// largest-magnitude entry is positive, ties broken by lowest index. Without
/// this the sign is arbitrary, and an arbitrary sign would make every medal
/// non-deterministic — see `spec/10-personality.md` stage 3.
pub fn symmetric_eigen(input: &Matrix) -> (Vec<f64>, Matrix) {
    assert_eq!(input.rows, input.cols, "eigen: matrix must be square");
    let n = input.rows;
    let mut a = input.clone();
    let mut v = Matrix::identity(n);

    for _sweep in 0..100 {
        // Convergence test: the sum of squares of the strict upper triangle.
        let mut off = 0.0;
        for p in 0..n {
            for q in (p + 1)..n {
                off += a.get(p, q) * a.get(p, q);
            }
        }
        if off <= 1e-24 {
            break;
        }

        for p in 0..n {
            for q in (p + 1)..n {
                let apq = a.get(p, q);
                if apq.abs() < 1e-18 {
                    continue;
                }
                // The rotation that annihilates a[p][q], via the stable root
                // of t² + 2θt − 1 = 0 (the smaller root, |t| ≤ 1).
                let theta = (a.get(q, q) - a.get(p, p)) / (2.0 * apq);
                let sign = if theta >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;

                for k in 0..n {
                    let akp = a.get(k, p);
                    let akq = a.get(k, q);
                    a.set(k, p, c * akp - s * akq);
                    a.set(k, q, s * akp + c * akq);
                }
                for k in 0..n {
                    let apk = a.get(p, k);
                    let aqk = a.get(q, k);
                    a.set(p, k, c * apk - s * aqk);
                    a.set(q, k, s * apk + c * aqk);
                }
                for k in 0..n {
                    let vkp = v.get(k, p);
                    let vkq = v.get(k, q);
                    v.set(k, p, c * vkp - s * vkq);
                    v.set(k, q, s * vkp + c * vkq);
                }
            }
        }
    }

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| {
        a.get(j, j)
            .partial_cmp(&a.get(i, i))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let values: Vec<f64> = order.iter().map(|&i| a.get(i, i)).collect();
    let mut vectors = Matrix::zeros(n, n);
    for (k, &src) in order.iter().enumerate() {
        // Canonical sign: largest-magnitude entry positive.
        let mut best = 0usize;
        for r in 1..n {
            if v.get(r, src).abs() > v.get(best, src).abs() + 1e-15 {
                best = r;
            }
        }
        let flip = if v.get(best, src) < 0.0 { -1.0 } else { 1.0 };
        for r in 0..n {
            vectors.set(r, k, flip * v.get(r, src));
        }
    }
    (values, vectors)
}

/// Project standardised rows onto the first `k` eigenvectors.
pub fn project(z: &Matrix, vectors: &Matrix, k: usize) -> Matrix {
    let mut out = Matrix::zeros(z.rows, k);
    for r in 0..z.rows {
        for c in 0..k {
            let mut s = 0.0;
            for d in 0..z.cols {
                s += z.get(r, d) * vectors.get(d, c);
            }
            out.set(r, c, s);
        }
    }
    out
}

/// Euclidean distance between two equal-length coordinate vectors.
pub fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}
