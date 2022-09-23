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

struct LessSwap<'a, T, L> {
    data: &'a mut [T],
    less: L,
}

struct InmutableLessSwap<'a, T, L> {
    data: &'a [T],
    less: L,
}

impl<'a, T, L> IndexSort for InmutableLessSwap<'a, T, L>
where
    L: Fn(usize, usize) -> bool,
{
    fn len(&self) -> usize {
        self.data.len()
    }

    fn swap(&mut self, _i: usize, _j: usize) {
        unreachable!()
    }

    fn less(&self, i: usize, j: usize) -> bool {
        (self.less)(i, j)
    }
}

impl<'a, T, L> IndexSort for LessSwap<'a, T, L>
where
    L: Fn(usize, usize) -> bool,
{
    fn len(&self) -> usize {
        self.data.len()
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.data.swap(i, j);
    }

    fn less(&self, i: usize, j: usize) -> bool {
        (self.less)(i, j)
    }
}

/// Golang's `sort.Slice`, `sort.SliceStable` and `sort.SliceIsSorted` in Rust
pub trait SliceSortExt {
    /// Item
    type Item;

    /// Slice sorts the slice x given the provided less function.
    ///
    /// The sort is not guaranteed to be stable: equal elements
    /// may be reversed from their original order.
    /// For a stable sort, use `slice_stable`.
    #[inline]
    fn slice<L>(data: &mut [Self::Item], less: L)
    where
        L: Fn(usize, usize) -> bool,
    {
        let mut sorter = LessSwap { data, less };
        sorter.sort()
    }

    /// Sorts the slice data using the provided less
    /// function, keeping equal elements in their original order.
    #[inline]
    fn slice_stable<L>(data: &mut [Self::Item], less: L)
    where
        L: Fn(usize, usize) -> bool,
    {
        let mut sorter = LessSwap { data, less };
        sorter.sort_stable()
    }

    /// Returns whether the slice x is sorted according to the provided less function.
    #[inline]
    fn slice_is_sorted<L>(data: &[Self::Item], less: L) -> bool
    where
        L: Fn(usize, usize) -> bool,
    {
        let sorter = InmutableLessSwap { data, less };
        sorter.is_sorted()
    }
}

impl<T> SliceSortExt for T {
    type Item = T;
}

/// Slice sorts the slice x given the provided less function.
///
/// The sort is not guaranteed to be stable: equal elements
/// may be reversed from their original order.
/// For a stable sort, use `slice_stable`.
#[inline]
pub fn slice<T, L>(data: &mut [T], less: L)
where
    L: Fn(usize, usize) -> bool,
{
    let mut sorter = LessSwap { data, less };
    sorter.sort()
}

/// Sorts the slice data using the provided less
/// function, keeping equal elements in their original order.
#[inline]
pub fn slice_stable<T, L>(data: &mut [T], less: L)
where
    L: Fn(usize, usize) -> bool,
{
    let mut sorter = LessSwap { data, less };
    sorter.sort_stable()
}

/// Returns whether the slice x is sorted according to the provided less function.
#[inline]
pub fn slice_is_sorted<T, L>(data: &[T], less: L) -> bool
where
    L: Fn(usize, usize) -> bool,
{
    let sorter = InmutableLessSwap { data, less };
    sorter.is_sorted()
}

/// Sort in reverse helper structure
pub struct Reverse<'a, T>(&'a mut T);

impl<'a, T: IndexSort> Reverse<'a, T> {
    pub fn new(t: &'a mut T) -> Self {
        Reverse(t)
    }

    pub fn into_inner(self) -> &'a mut T {
        self.0
    }
}

impl<'a, T: IndexSort> IndexSort for Reverse<'a, T> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.0.swap(i, j);
    }

    fn less(&self, i: usize, j: usize) -> bool {
        self.0.less(j, i)
    }
}

#[allow(clippy::len_without_is_empty)]
pub trait IndexSort {
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
    fn less(&self, i: usize, j: usize) -> bool;

    /// Swaps the elements with indexes i and j.
    fn swap(&mut self, i: usize, j: usize);

    /// Sort sorts data.
    /// It makes one call to data.Len to determine n and O(n*log(n)) calls to
    /// data.Less and data.Swap. The sort is not guaranteed to be stable.
    #[inline]
    fn sort(&mut self)
    where
        Self: Sized,
    {
        let n = self.len();
        quick_sort(self, 0, n, max_depth(n));
    }

    /// Notes on stable sorting:
    /// The used algorithms are simple and provable correct on all input and use
    /// only logarithmic additional stack space. They perform well if compared
    /// experimentally to other stable in-place sorting algorithms.
    ///
    /// Remarks on other algorithms evaluated:
    ///  - GCC's 4.6.3 stable_sort with merge_without_buffer from libstdc++:
    ///    Not faster.
    ///  - GCC's __rotate for block rotations: Not faster.
    ///  - "Practical in-place mergesort" from  Jyrki Katajainen, Tomi A. Pasanen
    ///    and Jukka Teuhola; Nordic Journal of Computing 3,1 (1996), 27-40:
    ///    The given algorithms are in-place, number of Swap and Assignments
    ///    grow as n log n but the algorithm is not stable.
    ///  - "Fast Stable In-Place Sorting with O(n) Data Moves" J.I. Munro and
    ///    V. Raman in Algorithmica (1996) 16, 115-160:
    ///    This algorithm either needs additional 2n bits or works only if there
    ///    are enough different elements available to encode some permutations
    ///    which have to be undone later (so not stable on any input).
    ///  - All the optimal in-place sorting/merging algorithms I found are either
    ///    unstable or rely on enough different elements in each step to encode the
    ///    performed block rearrangements. See also "In-Place Merging Algorithms",
    ///    Denham Coates-Evely, Department of Computer Science, Kings College,
    ///    January 2004 and the references in there.
    ///  - Often "optimal" algorithms are optimal in the number of assignments
    ///    but Interface has only Swap as operation.
    ///
    /// Stable sorts data in ascending order as determined by the Less method,
    /// while keeping the original order of equal elements.
    ///
    /// It makes one call to data.Len to determine n, O(n*log(n)) calls to
    /// data.Less and O(n*log(n)*log(n)) calls to data.Swap.
    #[inline]
    fn sort_stable(&mut self)
    where
        Self: Sized,
    {
        let n = self.len();
        stable(self, n);
    }

    /// Returns whether the data is sorted.
    #[inline]
    fn is_sorted(&self) -> bool {
        let n = self.len();
        for i in (1..n).rev() {
            if self.less(i, i - 1) {
                return false;
            }
        }
        true
    }

    /// Returns the reverse order for data.
    #[inline]
    fn reverse(&mut self) -> Reverse<Self>
    where
        Self: Sized,
    {
        Reverse(self)
    }
}

#[inline]
fn swap_slice<T>(data: &mut [T], i: usize, j: usize) {
    data.swap(i, j)
}

#[inline]
const fn swap_len<T>(data: &[T]) -> usize {
    data.len()
}

macro_rules! impl_index_sort_for_numbers {
    ($($t:ty)*) => ($(
        impl IndexSort for ::alloc::vec::Vec<$t> {
            fn len(&self) -> usize {
                ::alloc::vec::Vec::len(self)
            }

            fn less(&self, i: usize, j: usize) -> bool {
                self[i] < self[j]
            }

            fn swap(&mut self, i: usize, j: usize) {
                swap_slice(self, i, j);
            }
        }

        impl<'a> IndexSort for &'a mut ::alloc::vec::Vec<$t> {
            fn len(&self) -> usize {
                ::alloc::vec::Vec::len(self)
            }

            fn less(&self, i: usize, j: usize) -> bool {
                self[i] < self[j]
            }

            fn swap(&mut self, i: usize, j: usize) {
                swap_slice(self, i, j);
            }
        }

        impl<'a> IndexSort for &'a mut [$t] {
            fn len(&self) -> usize {
                swap_len(self)
            }

            fn less(&self, i: usize, j: usize) -> bool {
                self[i] < self[j]
            }

            fn swap(&mut self, i: usize, j: usize) {
                swap_slice(self, i, j);
            }
        }
    )*);
}

impl_index_sort_for_numbers! {
    u8 u16 u32 u64 u128 usize
    i8 i16 i32 i64 i128 isize
}

macro_rules! impl_index_sort_for_floats {
    ($($t:ty)*) => ($(
        impl IndexSort for ::alloc::vec::Vec<$t> {
            fn len(&self) -> usize {
                ::alloc::vec::Vec::len(self)
            }

            fn less(&self, i: usize, j: usize) -> bool {
                self[i] < self[j] || (self[i].is_nan() && !self[j].is_nan())
            }

            fn swap(&mut self, i: usize, j: usize) {
                swap_slice(self, i, j);
            }
        }

        impl<'a> IndexSort for &'a mut ::alloc::vec::Vec<$t> {
            fn len(&self) -> usize {
                ::alloc::vec::Vec::len(self)
            }

            fn less(&self, i: usize, j: usize) -> bool {
                self[i] < self[j] || (self[i].is_nan() && !self[j].is_nan())
            }

            fn swap(&mut self, i: usize, j: usize) {
                swap_slice(self, i, j);
            }
        }

        impl<'a> IndexSort for &'a mut [$t] {
            fn len(&self) -> usize {
                swap_len(self)
            }

            fn less(&self, i: usize, j: usize) -> bool {
                self[i] < self[j] || (self[i].is_nan() && !self[j].is_nan())
            }

            fn swap(&mut self, i: usize, j: usize) {
                swap_slice(self, i, j);
            }
        }
    )*);
}

impl_index_sort_for_floats! {
    f32 f64
}

/// Sorts data[a:b] using insertion sort.
#[inline]
fn insertion_sort(data: &mut impl IndexSort, a: usize, b: usize) {
    for i in a + 1..b {
        for j in (a + 1..=i).rev() {
            if data.less(j, j - 1) {
                data.swap(j, j - 1);
            } else {
                break;
            }
        }
    }
}

/// Implements the heap property on data[lo:hi].
/// first is an offset into the array where the root of the heap lies.
#[inline]
fn sift_down(data: &mut impl IndexSort, lo: usize, hi: usize, first: usize) {
    let mut root = lo;
    loop {
        let mut child = 2 * root + 1;
        if child >= hi {
            break;
        }

        if child + 1 < hi && data.less(first + child, first + child + 1) {
            child += 1;
        }

        if !data.less(first + root, first + child) {
            return;
        }

        data.swap(first + root, first + child);
        root = child;
    }
}

#[inline]
fn heap_sort(data: &mut impl IndexSort, a: usize, b: usize) {
    let first = a;
    let lo = 0;
    let hi = b - a;

    // Build heap with greatest element at top.
    let mut i = (hi - 1) / 2;
    loop {
        sift_down(data, i, hi, first);
        match i.checked_sub(1) {
            Some(v) => i = v,
            None => break,
        }
    }

    // Pop elements, largest first, into end of data.
    let mut i = hi - 1;
    loop {
        data.swap(first, first + i);
        sift_down(data, lo, i, first);
        match i.checked_sub(1) {
            Some(v) => i = v,
            None => break,
        }
    }
}

#[inline]
fn median_of_three(data: &mut impl IndexSort, m1: usize, m0: usize, m2: usize) {
    // sort 3 elements
    if data.less(m1, m0) {
        data.swap(m1, m0);
    }

    // data[m0] <= data[m1]
    if data.less(m2, m1) {
        data.swap(m2, m1);
        // data[m0] <= data[m2] && data[m1] < data[m2]
        if data.less(m1, m0) {
            data.swap(m1, m0);
        }
    }
    // now data[m0] <= data[m1] <= data[m2]
}

#[inline]
fn swap_range(data: &mut impl IndexSort, a: usize, b: usize, n: usize) {
    for i in 0..n {
        data.swap(a + i, b + i);
    }
}

#[inline]
fn do_pivot(data: &mut impl IndexSort, lo: usize, hi: usize) -> (usize, usize) {
    let m = (lo + hi) >> 1;
    if hi - lo > 40 {
        // Tukey ninther, median of three medians of three
        let s = (hi - lo) / 8;
        median_of_three(data, lo, lo + s, lo + 2 * s);
        median_of_three(data, m, m - s, m + s);
        median_of_three(data, hi - 1, hi - 1 - s, hi - 1 - 2 * s);
    }
    median_of_three(data, lo, m, hi - 1);

    // Invariants are:
    //	data[lo] = pivot (set up by ChoosePivot)
    //	data[lo < i < a] < pivot
    //	data[a <= i < b] <= pivot
    //	data[b <= i < c] unexamined
    //	data[c <= i < hi-1] > pivot
    //	data[hi-1] >= pivot
    let pivot = lo;
    let (mut a, mut c) = (lo + 1, hi - 1);
    while a < c && data.less(a, pivot) {
        a += 1;
    }
    let mut b = a;
    loop {
        // data[b] <= pivot
        while b < c && !data.less(pivot, b) {
            b += 1;
        }
        // data[c-1] > pivot
        while b < c && data.less(pivot, c - 1) {
            c -= 1;
        }
        if b >= c {
            break;
        }
        // data[b] > pivot; data[c-1] <= pivot
        data.swap(b, c - 1);
        b += 1;
        c -= 1;
    }

    // If hi-c<3 then there are duplicates (by property of median of nine).
    // Let's be a bit more conservative, and set border to 5.
    let mut protect = hi - c < 5;
    if !protect && hi - c < (hi - lo) / 4 {
        // Lets test some points for equality to pivot
        let mut dups = 0;
        // data[hi-1] = pivot
        if !data.less(pivot, hi - 1) {
            data.swap(c, hi - 1);
            c += 1;
            dups += 1;
        }

        // data[b-1] = pivot
        if !data.less(b - 1, pivot) {
            b -= 1;
            dups += 1;
        }

        // m-lo = (hi-lo)/2 > 6
        // b-lo > (hi-lo)*3/4-1 > 8
        // ==> m < b ==> data[m] <= pivot
        if !data.less(m, pivot) {
            data.swap(m, b - 1);
            b -= 1;
            dups += 1;
        }

        // if at least 2 points are equal to pivot, assume skewed distribution
        protect = dups > 1;
    }

    if protect {
        // Protect against a lot of duplicates
        // Add invariant:
        //	data[a <= i < b] unexamined
        //	data[b <= i < c] = pivot
        loop {
            // data[b] == pivot
            while a < b && !data.less(b - 1, pivot) {
                b -= 1;
            }
            // data[a] < pivot
            while a < b && !data.less(a, pivot) {
                a += 1;
            }
            if a >= b {
                break;
            }
            // data[a] == pivot; data[b-1] < pivot
            data.swap(a, b - 1);
            a += 1;
            b -= 1;
        }
    }

    // Swap pivot into middle
    data.swap(pivot, b - 1);
    (b - 1, c)
}

#[inline]
fn quick_sort(data: &mut impl IndexSort, mut a: usize, mut b: usize, mut max_depth: usize) {
    while b - a > 12 {
        if max_depth == 0 {
            heap_sort(data, a, b);
            return;
        }

        max_depth -= 1;
        let (mlo, mhi) = do_pivot(data, a, b);
        // Avoiding recursion on the larger subproblem guarantees
        // a stack depth of at most lg(b-a).
        if mlo - a < b - mhi {
            quick_sort(data, a, mlo, max_depth);
            a = mhi; // i.e., quickSort(data, mhi, b)
        } else {
            quick_sort(data, mhi, b, max_depth);
            b = mlo; // i.e., quickSort(data, a, mlo)
        }
    }

    if b - a > 1 {
        // Do ShellSort pass with gap 6
        // It could be written in this simplified form cause b-a <= 12
        for i in a + 6..b {
            if data.less(i, i - 6) {
                data.swap(i, i - 6);
            }
        }
        insertion_sort(data, a, b)
    }
}

#[inline]
fn stable(data: &mut impl IndexSort, n: usize) {
    let mut block_size = 20;
    let (mut a, mut b) = (0, block_size);
    while b <= n {
        insertion_sort(data, a, b);
        a = b;
        b += block_size;
    }

    insertion_sort(data, a, n);

    while block_size < n {
        (a, b) = (0, 2 * block_size);
        while b <= n {
            syn_merge(data, a, a + block_size, b);
            a = b;
            b += 2 * block_size;
        }
        if a + block_size < n {
            syn_merge(data, a, a + block_size, n);
        }
        block_size *= 2;
    }
}

/// Merges the two sorted subsequences data[a:m] and data[m:b] using
/// the SymMerge algorithm from Pok-Son Kim and Arne Kutzner, "Stable Minimum
/// Storage Merging by Symmetric Comparisons", in Susanne Albers and Tomasz
/// Radzik, editors, Algorithms - ESA 2004, volume 3221 of Lecture Notes in
/// Computer Science, pages 714-723. Springer, 2004.
///
/// Let M = m-a and N = b-n. Wolog M < N.
/// The recursion depth is bound by ceil(log(N+M)).
/// The algorithm needs O(M*log(N/M + 1)) calls to data.Less.
/// The algorithm needs O((M+N)*log(M)) calls to data.Swap.
///
/// The paper gives O((M+N)*log(M)) as the number of assignments assuming a
/// rotation algorithm which uses O(M+N+gcd(M+N)) assignments. The argumentation
/// in the paper carries through for Swap operations, especially as the block
/// swapping rotate uses only O(M+N) Swaps.
///
/// symMerge assumes non-degenerate arguments: a < m && m < b.
/// Having the caller check this condition eliminates many leaf recursion calls,
/// which improves performance.
#[inline]
fn syn_merge(data: &mut impl IndexSort, a: usize, m: usize, b: usize) {
    // Avoid unnecessary recursions of symMerge
    // by direct insertion of data[a] into data[m:b]
    // if data[a:m] only contains one element.
    if m - a == 1 {
        // Use binary search to find the lowest index i
        // such that data[i] >= data[a] for m <= i < b.
        // Exit the search loop with i == b in case no such index exists.
        let mut i = m;
        let mut j = b;
        while i < j {
            let h = (i + j) >> 1;
            if data.less(a, h) {
                i = h + 1;
            } else {
                j = h;
            }
        }

        // Swap values until data[a] reaches the position before i.
        for k in a..i - 1 {
            data.swap(k, k + 1);
        }
        return;
    }

    // Avoid unnecessary recursions of sym_merge
    // by direct insertion of data[m] into data[a:m]
    // if data[m:b] only contains one element.
    if b - m == 1 {
        // Use binary search to find the lowest index i
        // such that data[i] > data[m] for a <= i < m.
        // Exit the search loop with i == m in case no such index exists.
        let mut i = a;
        let mut j = m;
        while i < j {
            let h = (i + j) >> 1;
            if !data.less(m, h) {
                i = h + 1;
            } else {
                j = h;
            }
        }

        // Swap values until data[m] reaches the position i.
        for k in (i + 1..m + 1).rev() {
            data.swap(k, k - 1);
        }
        return;
    }

    let mid = (a + b) >> 1;
    let n = mid + m;
    let (mut start, mut r) = if m > mid { (n - b, mid) } else { (a, m) };

    let p = n - 1;
    while start < r {
        let c = (start + r) >> 1;
        if !data.less(c, p - c) {
            start = c + 1;
        } else {
            r = c;
        }
    }

    let end = n - start;
    if start < m && m < end {
        rotate(data, start, m, end);
    }

    if a < start && start < mid {
        syn_merge(data, a, start, mid);
    }

    if mid < end && end < b {
        syn_merge(data, mid, end, b);
    }
}

/// Rotates two consecutive blocks u = data[a:m] and v = data[m:b] in data:
/// Data of the form 'x u v y' is changed to 'x v u y'.
/// rotate performs at most b-a many calls to data.Swap,
/// and it assumes non-degenerate arguments: a < m && m < b.
#[inline]
fn rotate(data: &mut impl IndexSort, a: usize, m: usize, b: usize) {
    let mut i = m - a;
    let mut j = b - m;

    while i != j {
        if i > j {
            swap_range(data, m - 1, m, j);
            i -= j;
        } else {
            swap_range(data, m - i, m + j - i, i);
            j -= i;
        }
    }

    // i == j
    swap_range(data, m - i, m, i);
}

/// Returns a threshold at which quicksort should switch
/// to heapsort. It returns 2*ceil(lg(n+1)).
#[inline]
fn max_depth(n: usize) -> usize {
    let mut depth = 0;
    let mut i = n;
    while i > 0 {
        depth += 1;
        i >>= 1;
    }
    depth * 2
}
