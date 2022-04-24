use std::fmt::Debug;

use crate::objects::Type;
use crate::id::Id;

#[derive(Debug)]
pub enum PackfileType {
    Plain(Type),
    OffsetDelta((u64, Vec<u8>)),
    RefDelta((Id, Vec<u8>))
}
