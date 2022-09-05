use crate::HashMap;
use vela_utils::map_cell::MapCell;

lazy_static::lazy_static! {
    pub static ref BLOOM_HITS: MapCell<&'static str, usize> = {
        let mut map = HashMap::with_capacity(10);
        map.insert(DOES_NOT_HAVE_ALL, 0);
        map.insert(DOES_NOT_HAVE_HIT, 0);
        MapCell::new("bloom_hits_total", map)
    };
}

pub(crate) const DOES_NOT_HAVE_ALL: &str = "DOES_NOT_HAVE_ALL";

pub(crate) const DOES_NOT_HAVE_HIT: &str = "DOES_NOT_HAVE_HIT";
