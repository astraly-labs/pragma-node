use std::collections::HashMap;

pub enum StarkNetType {
    SimpleType { name: String, r#type: String },
    MerkleType(StarkNetMerkleType),
}

pub struct StarkNetMerkleType {
    pub name: String,
    pub r#type: String, // Using 'r#type' because 'type' is a reserved keyword in Rust
    pub contains: String,
}

impl StarkNetMerkleType {
    pub fn new() -> Self {
        StarkNetMerkleType {
            name: String::new(),
            r#type: "merkletree".to_string(),
            contains: String::new(),
        }
    }
}

pub struct StarkNetDomain {
    pub name: Option<String>,
    pub version: Option<String>,
    pub chain_id: Option<ChainId>,
}

pub enum ChainId {
    String(String),
    Number(u64),
}

pub struct TypedData {
    pub types: HashMap<String, Vec<StarkNetType>>,
    pub primary_type: String,
    pub domain: StarkNetDomain,
    pub message: HashMap<String, String>, // Assuming the message is a map of string to string for simplicity
}
