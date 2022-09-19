#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "ref_counter")]
extern crate alloc;
#[cfg(feature = "ref_counter")]
pub mod ref_counter;

#[cfg(feature = "map_cell")]
pub mod map_cell;

/// Golang `sort.Search` in Rust
pub fn binary_search<F: FnMut(isize) -> bool>(target: isize, mut op: F) -> isize {
    // Define f(-1) == false and f(n) == true.
    // Invariant: f(i-1) == false, f(j) == true.
    let (mut i, mut j) = (0, target);
    while i < j {
        let h = (((i + j) as usize) >> 1) as isize; // avoid overflow when computing h
                                                    // i ≤ h < j
        if !op(h) {
            i = h + 1; // preserves f(i-1) == false
        } else {
            j = h; // preserves f(j) == true
        }
    }

    // i == j, f(i-1) == false, and f(j) (= f(i)) == true  =>  answer is i.
    i
}

pub fn binary_search_by_usize<F>(n: usize, mut f: F) -> usize
where
    F: FnMut(usize) -> bool,
{
    let mut i = 0;
    let mut j = n;
    while i < j {
        let h = (i + j) / 2;
        if !f(h) {
            i = h + 1;
        } else {
            j = h;
        }
    }
    i
}

pub trait GoSort {
    /// Len is the number of elements in the collection.
    fn len(&self) -> usize;

    /// Less reports whether the element with index i
    /// must sort before the element with index j.
    ///
    /// If both Less(i, j) and Less(j, i) are false,
    /// then the elements at index i and j are considered equal.
    /// Sort may place equal elements in any order in the final result,
    /// while Stable preserves the original input order of equal elements.
    ///
    /// Less must describe a transitive ordering:
    ///  - if both less(i, j) and Less(j, k) are true, then Less(i, k) must be true as well.
    ///  - if both less(i, j) and less(j, k) are false, then less(i, k) must be false as well.
    fn less(&mut self, i: usize, j: usize) -> bool;

    /// Swaps the elements with indexes i and j.
    fn swap(&mut self, i: usize, j: usize);
}
