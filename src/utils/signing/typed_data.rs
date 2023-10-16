use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use starknet::core::{
    crypto::compute_hash_on_elements,
    types::FieldElement,
    utils::{cairo_short_string_to_felt, get_selector_from_name},
};
use std::fs;
use std::path::Path;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};
use std::io::Read;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StarkNetDomain {
    pub name: Option<String>,
    pub version: Option<String>,
    pub chain_id: Option<Value>, // Using Value to represent either String or Number
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Parameter {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ChainId {
    String(String),
    Number(u64),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DomainType {
  name: String,
  version: String,
  #[serde(rename = "chainId")]
  chain_id: ChainId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypedData {
    #[serde(rename = "types")]
    types: HashMap<String, Vec<Parameter>>,
    #[serde(rename = "primaryType")]
    primary_type: String,
    #[serde(rename = "domain")]
    domain: DomainType,
    #[serde(rename = "message")]
    message: HashMap<String, String>,
}

impl TypedData {
    /// Returns true if the type is a struct.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type to check.
    fn is_struct(&self, type_name: &str) -> bool {
        self.types.contains_key(type_name)
    }

    /// Encodes a value according to the type.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type to encode.
    /// * `value` - The value to encode.
    ///
    /// # Returns
    ///
    /// * The encoded value.
    fn encode_value(&self, type_name: &str, value: &Value) -> Result<FieldElement, &'static str> {
        if Self::is_pointer(type_name) {
            if let Value::Array(arr) = value {
                let type_name = Self::strip_pointer(type_name);

                if self.is_struct(&type_name) {
                    // Assuming you have a method called `struct_hash` similar to the Python version
                    let hashes: Vec<FieldElement> = arr
                        .iter()
                        .map(|data| FieldElement::from_str(&get_hex(data).unwrap()).unwrap())
                        .collect();
                    // Assuming you have a method called `compute_hash_on_elements`
                    return Ok(compute_hash_on_elements(&hashes));
                } else {
                    let hashes: Vec<FieldElement> = arr
                        .iter()
                        .map(|val| FieldElement::from_str(&get_hex(val).unwrap()).unwrap())
                        .collect();
                    return Ok(compute_hash_on_elements(&hashes));
                }
            } else {
                return Err("Expected a list for pointer type");
            }
        } else if self.is_struct(type_name) {
            if let Value::Object(obj) = value {
                return Ok(self.struct_hash(type_name, obj));
            } else {
                return Err("Expected an object for struct type");
            }
        } else {
            return Ok(FieldElement::from_str(&get_hex(value).unwrap()).unwrap());
        }
    }

    /// Collects all the dependencies of a type.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type to collect dependencies for.
    /// * `types` - A map of all the types.
    /// * `dependencies` - A mutable set of all the dependencies.
    fn collect_deps(
        type_name: &str,
        types: &HashMap<String, Vec<Parameter>>,
        dependencies: &mut HashSet<String>,
    ) {
        for param in types.get(type_name).unwrap() {
            let fixed_type = Self::strip_pointer(&param.type_);
            if types.contains_key(&fixed_type) && !dependencies.contains(&fixed_type) {
                dependencies.insert(fixed_type.clone());
                // recursive call
                Self::collect_deps(&fixed_type, types, dependencies);
            }
        }
    }

    /// Returns all the dependencies of a type.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type to get dependencies for.
    ///
    /// # Returns
    ///
    /// * A vector of all the dependencies.
    fn get_dependencies(&self, type_name: &str) -> Vec<String> {
        if !self.types.contains_key(type_name) {
            // type_name is a primitive type, has no dependencies
            return vec![];
        }

        let mut dependencies = HashSet::new();

        // collect dependencies into a set
        Self::collect_deps(type_name, &self.types, &mut dependencies);
        let mut result = vec![type_name.to_string()];
        result.extend(dependencies.into_iter());
        result
    }

    /// Encodes a type.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type to encode.
    ///
    /// # Returns
    ///
    /// * The encoded type.
    pub fn encode_type(&self, type_name: &str) -> String {
        let mut dependencies = self.get_dependencies(type_name);
        let primary = dependencies.remove(0);
        dependencies.sort();

        let types = std::iter::once(primary)
            .chain(dependencies.into_iter())
            .collect::<Vec<_>>();

        types
            .into_iter()
            .map(|dependency| {
                let lst: Vec<String> = self
                    .types
                    .get(&dependency)
                    .unwrap()
                    .iter()
                    .map(|t| format!("{}:{}", t.name, t.type_))
                    .collect();
                format!("{}({})", dependency, lst.join(","))
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Encodes the data of a type.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type to encode.
    /// * `data` - The data to encode.
    ///
    /// # Returns
    ///
    /// * The encoded data.
    fn encode_data(&self, type_name: &str, data: &Map<String, Value>) -> Vec<FieldElement> {
        self.types[type_name]
            .iter()
            .map(|param| self.encode_value(&param.type_, &data[&param.name]).unwrap())
            .collect()
    }

    /// Computes the hash of a struct.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type to hash.
    /// * `data` - The data to hash.
    ///
    /// # Returns
    ///
    /// * The hash of the struct.
    pub fn struct_hash(&self, type_name: &str, data: &Map<String, Value>) -> FieldElement {
        let type_hash = self.type_hash(type_name);
        let encoded_data = self
            .encode_data(type_name, data)
            .iter()
            .cloned()
            .collect::<Vec<FieldElement>>();
        let elements = std::iter::once(type_hash)
            .chain(encoded_data.into_iter())
            .collect::<Vec<FieldElement>>();

        compute_hash_on_elements(&elements)
    }

    /// Computes the hash of a type.
    ///
    /// # Arguments
    ///
    /// * `type_name` - The name of the type to hash.
    ///
    /// # Returns
    ///
    /// * The hash of the type.
    pub fn type_hash(&self, type_name: &str) -> FieldElement {
        get_selector_from_name(&self.encode_type(type_name)).unwrap()
    }

    /// Computes the hash of the message.
    ///
    /// # Arguments
    ///
    /// * `account_address` - Address of an account.
    ///
    /// # Returns
    ///
    /// * The hash of the message.
    pub fn message_hash(&self, account_address: FieldElement) -> FieldElement {
        let prefix = FieldElement::from_str("StarkNet Domain").unwrap();

        let json_str = serde_json::to_string(&self.domain).unwrap();
        let json_map: Map<String, Value> = serde_json::from_str(&json_str).unwrap();
        let message = self.struct_hash("StarkNetDomain", &json_map);

        let json_str = serde_json::to_string(&self.message).unwrap();
        let json_map: Map<String, Value> = serde_json::from_str(&json_str).unwrap();
        let message_hash = self.struct_hash(&self.primary_type, &json_map);

        compute_hash_on_elements(&[prefix, message, account_address, message_hash])
    }

    fn is_pointer(value: &str) -> bool {
        value.ends_with('*')
    }

    fn strip_pointer(value: &str) -> String {
        if Self::is_pointer(value) {
            value[..value.len() - 1].to_string()
        } else {
            value.to_string()
        }
    }
}

/// Computes the hex string of a json value.
///
/// # Arguments
///
/// * `value` - The value to convert to hex.
///
/// # Returns
///
/// * The hex string of the value.
pub(crate) fn get_hex(value: &Value) -> Result<String, &'static str> {
    println!("value: {:?}", value);
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(format!("{:#x}", i))
            } else {
                Err("Number is not an i64")
            }
        }
        Value::String(s) => {
            if s.starts_with("0x") {
                Ok(s.clone())
            } else if s.chars().all(|c| c.is_numeric()) {
                Ok(format!("{:#x}", s.parse::<i64>().unwrap()))
            } else {
                Ok(format!("{:#x}", cairo_short_string_to_felt(s).unwrap()))
            }
        }
        _ => Err("Unsupported value type for get_hex"),
    }
}

const TYPED_DATA_DIR: &str = "src/utils/signing/mock"; // Update this to your actual directory path.

pub(crate) fn load_typed_data(file_name: &str) -> TypedData {
    let file_path = format!("{}/{}", TYPED_DATA_DIR, file_name);
    let path = Path::new(&file_path);
    let mut file = fs::File::open(&path).expect("Error opening the file");
    let mut buff = String::new();
    file.read_to_string(&mut buff).unwrap();
    let typed_data: TypedData = serde_json::from_str(&buff).expect("Error parsing the JSON");
    typed_data
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use super::*;

    #[rstest]
    #[case(json!(123), "0x7b")]
    #[case(json!("123"), "0x7b")]
    #[case(json!("0x7b"), "0x7b")]
    #[case(json!("short_string"), "0x73686f72745f737472696e67")]
    fn test_get_hex(#[case] value: Value, #[case] result: &str) {
        assert_eq!(get_hex(&value).unwrap(), result);
    }

    const TD: &str = "typed_data_example.json";
    const TD_STRING: &str = "typed_data_long_string_example.json";
    const TD_FELT_ARR: &str = "typed_data_felt_array_example.json";
    const TD_STRUCT_ARR: &str = "typed_data_struct_array_example.json";

    #[rstest]
    #[case(
        TD,
        "Mail",
        "Mail(from:Person,to:Person,contents:felt)Person(name:felt,wallet:felt)"
    )]
    #[case(
        TD_FELT_ARR,
        "Mail",
        "Mail(from:Person,to:Person,felts_len:felt,felts:felt*)Person(name:felt,wallet:felt)"
    )]
    #[case(TD_STRING, "Mail", "Mail(from:Person,to:Person,contents:String)Person(name:felt,wallet:felt)String(len:felt,data:felt*)")]
    #[case(TD_STRUCT_ARR, "Mail", "Mail(from:Person,to:Person,posts_len:felt,posts:Post*)Person(name:felt,wallet:felt)Post(title:felt,content:felt)")]
    fn test_encode_type(
        #[case] example: &str,
        #[case] type_name: &str,
        #[case] encoded_type: &str,
    ) {
        println!("example: {:?}", example);
        println!("type_name: {:?}", type_name);
        println!("encoded_type: {:?}", encoded_type);
        let typed_data = load_typed_data(example);
        let res = typed_data.encode_type(type_name);
        assert_eq!(res, encoded_type);
    }
}
