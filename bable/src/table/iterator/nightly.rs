
use super::*;

pub trait TableIterator {
    type Key<'a>
    where
        Self: 'a;

    type Value<'a>
        where
            Self: 'a;
    
    
}
