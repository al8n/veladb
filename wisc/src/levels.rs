mod handler;
use core::sync::atomic::{AtomicI64, AtomicU64};

pub use handler::*;

use kvstructs::KeyRange;

struct LevelsController {
    next_file_id: AtomicU64,
    l0_stalls_ms: AtomicI64,
    //
    // levels: ,
}

pub(crate) struct CompactDef<L, R> {
    pub(crate) this_level: LevelHandler,
    pub(crate) next_level: LevelHandler,
    pub(crate) this_range: KeyRange<L, R>,
    pub(crate) next_range: KeyRange<L, R>,
    pub(crate) top: Vec<bable::Table>,
    pub(crate) bot: Vec<bable::Table>,
}
