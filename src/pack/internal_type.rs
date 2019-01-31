use crate::id::Id;

#[derive(Debug)]
pub enum PackfileType {
    Plain(u8),
    OffsetDelta((u64, Vec<u8>)),
    RefDelta((Id, Vec<u8>))
}
