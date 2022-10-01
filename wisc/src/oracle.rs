use vela_utils::watermark::Watermark;

pub struct WiscOracle {}

impl vela_traits::Oracle for WiscOracle {
    fn read_timestamp(&self) -> u64 {
        unimplemented!()
    }
}
