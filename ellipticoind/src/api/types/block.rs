use crate::api::types::misc::{Bytes, U32};
use crate::api::types::transaction::Transaction;
use crate::models;

#[derive(Clone, Debug)]
pub struct Block {
    pub number: U32,
    pub transactions: Vec<Transaction>,
    pub sealed: bool,
    pub memory_changeset_hash: Bytes,
    pub storage_changeset_hash: Bytes,
}

#[juniper::graphql_object]
impl Block {
    fn number(&self) -> U32 {
        self.number.clone()
    }

    fn transactions(&self) -> Vec<Transaction> {
        self.transactions.clone()
    }

    fn sealed(&self) -> bool {
        self.sealed
    }

    fn memory_changeset_hash(&self) -> Bytes {
        self.memory_changeset_hash.clone()
    }

    fn storage_changeset_hash(&self) -> Bytes {
        self.storage_changeset_hash.clone()
    }
}

#[derive(juniper::GraphQLEnum, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GraphQLPostBlockResultStatus {
    NotConsidered,
    Rejected,
    Witnessed,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct GraphQLPostBlockResult {
    pub status: GraphQLPostBlockResultStatus,
    pub proof: Option<Vec<Bytes>>,
}

#[juniper::graphql_object]
impl GraphQLPostBlockResult {
    fn status(&self) -> GraphQLPostBlockResultStatus {
        self.status.clone()
    }

    fn proof(&self) -> Option<Vec<Bytes>> {
        self.proof.as_ref().cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockResult {
    NotConsidered(),
    Rejected(Vec<Bytes>),
    Witnessed(Bytes),
}

impl From<(models::Block, Vec<models::Transaction>)> for Block {
    fn from(block: (models::Block, Vec<models::Transaction>)) -> Block {
        Self {
            number: U32(block.0.number as u32),
            sealed: block.0.sealed,
            memory_changeset_hash: Bytes(block.0.memory_changeset_hash),
            storage_changeset_hash: Bytes(block.0.storage_changeset_hash),
            transactions: block
                .1
                .into_iter()
                .map(Transaction::from)
                .collect::<Vec<Transaction>>(),
        }
    }
}

impl From<BlockResult> for GraphQLPostBlockResult {
    fn from(res: BlockResult) -> Self {
        return match res {
            BlockResult::NotConsidered() => Self {
                status: GraphQLPostBlockResultStatus::NotConsidered,
                proof: None,
            },
            BlockResult::Rejected(x) => Self {
                status: GraphQLPostBlockResultStatus::Rejected,
                proof: Some(x.clone()),
            },
            BlockResult::Witnessed(x) => Self {
                status: GraphQLPostBlockResultStatus::Witnessed,
                proof: Some(vec![x.clone(); 1]),
            },
        };
    }
}

impl From<GraphQLPostBlockResult> for BlockResult {
    fn from(res: GraphQLPostBlockResult) -> Self {
        return match res.status {
            GraphQLPostBlockResultStatus::NotConsidered => BlockResult::NotConsidered(),
            GraphQLPostBlockResultStatus::Rejected => {
                BlockResult::Rejected(res.proof.unwrap().clone())
            }
            GraphQLPostBlockResultStatus::Witnessed => {
                BlockResult::Witnessed(res.proof.unwrap().get(0).unwrap().clone())
            }
        };
    }
}
