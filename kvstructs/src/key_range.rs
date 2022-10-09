use crate::KeyExt;

/// key range
pub struct KeyRange<L, R> {
    left: L,
    right: R,
    inf: bool,
    // size is used for Key splits.
    size: u64,
}

impl<L: KeyExt, R: KeyExt, OL: KeyExt, OR: KeyExt> PartialEq<KeyRange<OL, OR>> for KeyRange<L, R> {
    fn eq(&self, other: &KeyRange<OL, OR>) -> bool {
        let self_l = self.left.as_key_ref();
        let self_r = self.right.as_key_ref();
        let other_l = other.left.as_key_ref();
        let other_r = other.right.as_key_ref();
        self_l == other_l && self_r == other_r && self.inf == other.inf
    }
}

impl<L, R> KeyRange<L, R> {
    ///
    #[inline]
    pub fn new(left: L, right: R, size: u64) -> Self {
        Self {
            left,
            right,
            inf: false,
            size,
        }
    }

    ///
    pub const fn start(&self) -> &L {
        &self.left
    }

    ///
    pub const fn end(&self) -> &R {
        &self.right
    }

    ///
    #[inline]
    pub const fn get_size(&self) -> u64 {
        self.size
    }

    ///
    #[inline]
    pub fn incr_size(&mut self, size: u64) {
        self.size += size;
    }

    ///
    #[inline]
    pub fn decr_size(&mut self, size: u64) {
        self.size -= size;
    }

    ///
    #[inline]
    pub fn set_size(&mut self, size: u64) {
        self.size = size;
    }
}

impl<L: KeyExt, R: KeyExt> KeyRange<L, R> {
    /// Returns the key range is empty or not.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.left.as_key_ref().is_empty() && self.right.as_key_ref().is_empty() && !self.inf
    }

    /// Extends a key range
    #[inline]
    pub fn extend(&mut self, kr: KeyRange<L, R>) {
        if kr.is_empty() {
            return;
        }

        if self.is_empty() {
            *self = kr;
            return;
        }

        let self_l = self.left.as_key_ref();
        let self_r = self.right.as_key_ref();
        let kr_l = kr.left.as_key_ref();
        let kr_r = kr.right.as_key_ref();

        if self_l.is_empty() || kr_l < self_l {
            self.left = kr.left;
        }

        if self_r.is_empty() || kr_r > self_r {
            self.right = kr.right;
        }

        if kr.inf {
            self.inf = true;
        }
    }

    /// Returns the key range overlaps with other key range.
    pub fn overlaps_with<OL: KeyExt, OR: KeyExt>(&self, other: &KeyRange<OL, OR>) -> bool {
        // Empty keyRange always overlaps.
        if self.is_empty() {
            return true;
        }

        // Empty dst doesn't overlap with anything.
        if other.is_empty() {
            return false;
        }

        if self.inf || other.inf {
            return true;
        }

        // [other.left, other.right] ... [self.left, self.right]
        // If my left is greater than other right, we have no overlap.
        let self_l = self.left.as_key_ref();
        let other_r = other.right.as_key_ref();
        if self_l > other_r {
            return false;
        }

        // [self.left, self.right] ... [other.left, other.right]
        // If my right is less than dst left, we have no overlap.
        let self_r = self.right.as_key_ref();
        let other_l = other.left.as_key_ref();
        if self_r < other_l {
            return false;
        }

        // We have overlap
        true
    }
}
