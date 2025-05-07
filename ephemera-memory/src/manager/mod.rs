mod manager;

pub use manager::*;

use serde::{Serialize, Deserialize};

use crate::MemoryFragment;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryArgs  {
    pub keywords: String,
    pub time_range: Option<TimeRange>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub memories: Vec<MemoryFragment>
}

#[allow(async_fn_in_trait)]
pub trait Manager {
    async fn append(&mut self, memory: &MemoryFragment) -> anyhow::Result<()>;
    async fn recall(&self, query: &QueryArgs) -> anyhow::Result<QueryResult>;
}