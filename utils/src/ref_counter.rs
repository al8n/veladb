/// `RefCounter<T>` is a very simple wrapper, you can treat this like
///
/// - `triomphe::Arc` in `std`
/// - `alloc::rc::Rc` in `no_std` with `rc` feature enabled
#[derive(Debug)]
#[repr(transparent)]
pub struct RefCounter<T: ?Sized> {
    #[cfg(not(all(not(feature = "std"), feature = "singlethread")))]
    ptr: triomphe::Arc<T>,

    #[cfg(all(not(feature = "std"), feature = "singlethread"))]
    ptr: alloc::rc::Rc<T>,
}

impl<T> Clone for RefCounter<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone(),
        }
    }
}

impl<T: ?Sized> core::ops::Deref for RefCounter<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<T: ?Sized> AsRef<T> for RefCounter<T> {
    fn as_ref(&self) -> &T {
        &self.ptr
    }
}

impl<T> RefCounter<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self {
            #[cfg(not(all(not(feature = "std"), feature = "singlethread")))]
            ptr: triomphe::Arc::new(val),
            #[cfg(all(not(feature = "std"), feature = "singlethread"))]
            ptr: alloc::rc::Rc::new(val),
        }
    }

    #[inline]
    pub fn count(ptr: &Self) -> usize {
        #[cfg(not(all(not(feature = "std"), feature = "singlethread")))]
        {
            triomphe::Arc::count(&ptr.ptr)
        }

        #[cfg(all(not(feature = "std"), feature = "singlethread"))]
        {
            alloc::rc::Rc::strong_count(&ptr.ptr)
        }
    }
}

impl<T> From<T> for RefCounter<T> {
    fn from(val: T) -> Self {
        Self::new(val)
    }
}
