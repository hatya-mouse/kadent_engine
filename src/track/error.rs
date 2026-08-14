use crate::graph::error::GraphError;

pub enum TrackError {
    GraphError(GraphError),
    ThreadSpawnFailed(String),
}
