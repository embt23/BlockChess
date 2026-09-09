//! The oracle for episode 06.
//!
//! `bc-hash` is checked against FIPS 180-4, `bc-sig` against RFC 8032,
//! `bc-chess` against published perft counts. The eigensolver's equivalent is
//! matrices whose spectra are known analytically, plus the two structural
//! identities any correct symmetric decomposition must satisfy.
//!
//! `docs/build-log.md` entry 02 is the reason this file exists: twelve
//! hand-written sanity tests passed a bug that RFC 8032 caught immediately.

use bc_style::linalg::{column_stats, covariance, standardise, symmetric_eigen, Matrix};

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

fn assert_vec_close(got: &[f64], want: &[f64], tol: f64) {
    assert_eq!(got.len(), want.len(), "length: {got:?} vs {want:?}");
    for (g, w) in got.iter().zip(want) {
        assert!(close(*g, *w, tol), "got {got:?}, want {want:?}");
    }
}

#[test]
fn diagonal_matrix_returns_its_diagonal_sorted() {
    let m = Matrix::from_rows(&[
        vec![3.0, 0.0, 0.0],
        vec![0.0, 7.0, 0.0],
        vec![0.0, 0.0, 5.0],
    ]);
    let (values, _) = symmetric_eigen(&m);
    assert_vec_close(&values, &[7.0, 5.0, 3.0], 1e-12);
}

#[test]
fn two_by_two_has_the_textbook_spectrum() {
    // [[2,1],[1,2]] → eigenvalues 3 and 1, eigenvectors (1,1)/√2 and (1,−1)/√2.
    let m = Matrix::from_rows(&[vec![2.0, 1.0], vec![1.0, 2.0]]);
    let (values, vectors) = symmetric_eigen(&m);
    assert_vec_close(&values, &[3.0, 1.0], 1e-12);

    let r = std::f64::consts::FRAC_1_SQRT_2;
    assert!(close(vectors.get(0, 0).abs(), r, 1e-12));
    assert!(close(vectors.get(1, 0).abs(), r, 1e-12));
    // The dominant eigenvector's components share a sign; the other's do not.
    assert!(vectors.get(0, 0) * vectors.get(1, 0) > 0.0);
    assert!(vectors.get(0, 1) * vectors.get(1, 1) < 0.0);
}

#[test]
fn tridiagonal_toeplitz_matches_its_closed_form() {
    // For the n×n matrix with 4 on the diagonal and 1 off it, the eigenvalues
    // are 4 + 2cos(kπ/(n+1)). At n = 3 that is 4+√2, 4, 4−√2.
    let m = Matrix::from_rows(&[
        vec![4.0, 1.0, 0.0],
        vec![1.0, 4.0, 1.0],
        vec![0.0, 1.0, 4.0],
    ]);
    let (values, _) = symmetric_eigen(&m);
    let s = 2.0f64.sqrt();
    assert_vec_close(&values, &[4.0 + s, 4.0, 4.0 - s], 1e-10);
}

#[test]
fn eigenvectors_are_orthonormal() {
    let m = Matrix::from_rows(&[
        vec![6.0, 2.0, 1.0],
        vec![2.0, 5.0, 3.0],
        vec![1.0, 3.0, 4.0],
    ]);
    let (_, v) = symmetric_eigen(&m);
    for i in 0..3 {
        for j in 0..3 {
            let dot: f64 = (0..3).map(|r| v.get(r, i) * v.get(r, j)).sum();
            let want = if i == j { 1.0 } else { 0.0 };
            assert!(close(dot, want, 1e-12), "VᵀV[{i}][{j}] = {dot}");
        }
    }
}

#[test]
fn decomposition_reconstructs_the_original() {
    let m = Matrix::from_rows(&[
        vec![6.0, 2.0, 1.0],
        vec![2.0, 5.0, 3.0],
        vec![1.0, 3.0, 4.0],
    ]);
    let (values, v) = symmetric_eigen(&m);
    for r in 0..3 {
        for c in 0..3 {
            let got: f64 = (0..3).map(|k| v.get(r, k) * values[k] * v.get(c, k)).sum();
            assert!(close(got, m.get(r, c), 1e-10), "VΛVᵀ[{r}][{c}] = {got}");
        }
    }
}

#[test]
fn trace_is_preserved() {
    let m = Matrix::from_rows(&[
        vec![6.0, 2.0, 1.0],
        vec![2.0, 5.0, 3.0],
        vec![1.0, 3.0, 4.0],
    ]);
    let (values, _) = symmetric_eigen(&m);
    let trace: f64 = (0..3).map(|i| m.get(i, i)).sum();
    assert!(close(values.iter().sum::<f64>(), trace, 1e-10));
}

/// The sign of an eigenvector is mathematically arbitrary, so the crate pins it:
/// largest-magnitude entry positive. Without this rule the medal for a given
/// player would differ between runs on different machines — the same class of
/// bug as a non-canonical encoding, and just as silent.
#[test]
fn eigenvector_signs_are_canonical() {
    let m = Matrix::from_rows(&[vec![2.0, 1.0], vec![1.0, 2.0]]);
    let (_, v) = symmetric_eigen(&m);
    for c in 0..2 {
        let col = v.col(c);
        let biggest = col
            .iter()
            .cloned()
            .fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
        assert!(biggest > 0.0, "column {c} is not sign-canonical: {col:?}");
    }
}

/// A constant column carries no information about any player. It must map to
/// zero rather than dividing by a zero deviation — which would produce NaN and
/// poison every downstream distance silently.
#[test]
fn constant_columns_do_not_produce_nan() {
    let m = Matrix::from_rows(&[vec![1.0, 5.0], vec![2.0, 5.0], vec![3.0, 5.0]]);
    let (mean, dev) = column_stats(&m);
    assert!(close(mean[1], 5.0, 1e-12));
    assert_eq!(dev[1], 0.0);

    let z = standardise(&m, &mean, &dev);
    assert!(z.data.iter().all(|v| v.is_finite()));
    for r in 0..3 {
        assert_eq!(z.get(r, 1), 0.0);
    }
    assert!(covariance(&z).data.iter().all(|v| v.is_finite()));
}

/// Standardised columns have mean zero and unit deviation, so the covariance
/// of standardised data is a correlation matrix with a unit diagonal.
#[test]
fn covariance_of_standardised_data_has_unit_diagonal() {
    let m = Matrix::from_rows(&[
        vec![1.0, 2.0],
        vec![2.0, 8.0],
        vec![3.0, 4.0],
        vec![4.0, 16.0],
    ]);
    let (mean, dev) = column_stats(&m);
    let z = standardise(&m, &mean, &dev);
    let c = covariance(&z);
    // n/(n−1) scaling: population deviation, sample covariance.
    let expect = 4.0 / 3.0;
    assert!(close(c.get(0, 0), expect, 1e-12), "{}", c.get(0, 0));
    assert!(close(c.get(1, 1), expect, 1e-12), "{}", c.get(1, 1));
    assert!(close(c.get(0, 1), c.get(1, 0), 1e-15));
}
