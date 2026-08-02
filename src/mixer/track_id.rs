use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Debug, Serialize, Deserialize)]
pub struct TrackID(pub u64);
