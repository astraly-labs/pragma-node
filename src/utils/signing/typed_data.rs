use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use starknet::core::{
    crypto::compute_hash_on_elements,
    types::FieldElement,
    utils::{cairo_short_string_to_felt, get_selector_from_name},
};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

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
pub struct DomainType {
    pub name: String,
    pub version: String,
    #[serde(rename = "chainId")]
    pub chain_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypedData<T: Serialize> {
    #[serde(rename = "types")]
    pub types: HashMap<String, Vec<Parameter>>,
    #[serde(rename = "primaryType")]
    pub primary_type: String,
    #[serde(rename = "domain")]
    pub domain: DomainType,
    #[serde(rename = "message")]
    pub message: T,
}

impl<T> TypedData<T>
where
    T: Serialize,
{
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
                        .map(|data| self.struct_hash(&type_name, data.as_object().unwrap()))
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
        let prefix = cairo_short_string_to_felt("StarkNet Message").unwrap();

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
    match value {
        Value::Number(n) => {
            let i = FieldElement::from_dec_str(&n.to_string()).expect("Error parsing number");
            Ok(format!("{:#x}", i))
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::fs;
    use std::io::Read;
    use std::path::Path;

    const TYPED_DATA_DIR: &str = "src/utils/signing/mock"; // Update this to your actual directory path.

    pub(crate) fn load_typed_data<T>(file_name: &str) -> TypedData<T>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let file_path = format!("{}/{}", TYPED_DATA_DIR, file_name);
        let path = Path::new(&file_path);
        let mut file = fs::File::open(&path).expect("Error opening the file");
        let mut buff = String::new();
        file.read_to_string(&mut buff).unwrap();
        let typed_data: TypedData<T> = serde_json::from_str(&buff).expect("Error parsing the JSON");
        typed_data
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct WalletType {
        name: String,
        wallet: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct MailType {
        from: WalletType,
        to: WalletType,
        contents: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct MailTypeFeltArray {
        from: WalletType,
        to: WalletType,
        felts_len: u64,
        felts: Vec<u64>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ContentsType {
        len: u64,
        data: Vec<FieldElement>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct MailTypeLongString {
        from: WalletType,
        to: WalletType,
        contents: ContentsType,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Post {
        title: String,
        content: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct MailTypeStructArray {
        from: WalletType,
        to: WalletType,
        posts_len: u64,
        posts: Vec<Post>,
    }

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
    fn test_encode_type_simple(
        #[case] example: &str,
        #[case] type_name: &str,
        #[case] encoded_type: &str,
    ) {
        let typed_data: TypedData<MailType> = load_typed_data(example);
        let res = typed_data.encode_type(type_name);
        assert_eq!(res, encoded_type);
    }

    #[rstest]
    #[case(
        TD_FELT_ARR,
        "Mail",
        "Mail(from:Person,to:Person,felts_len:felt,felts:felt*)Person(name:felt,wallet:felt)"
    )]
    fn test_encode_type_felt_array(
        #[case] example: &str,
        #[case] type_name: &str,
        #[case] encoded_type: &str,
    ) {
        let typed_data: TypedData<MailTypeFeltArray> = load_typed_data(example);
        let res = typed_data.encode_type(type_name);
        assert_eq!(res, encoded_type);
    }

    #[rstest]
    #[case(TD_STRING, "Mail", "Mail(from:Person,to:Person,contents:String)Person(name:felt,wallet:felt)String(len:felt,data:felt*)")]
    fn test_encode_type_long_string(
        #[case] example: &str,
        #[case] type_name: &str,
        #[case] encoded_type: &str,
    ) {
        let typed_data: TypedData<MailTypeLongString> = load_typed_data(example);
        let res = typed_data.encode_type(type_name);
        assert_eq!(res, encoded_type);
    }

    #[rstest]
    #[case(TD_STRUCT_ARR, "Mail", "Mail(from:Person,to:Person,posts_len:felt,posts:Post*)Person(name:felt,wallet:felt)Post(title:felt,content:felt)")]
    fn test_encode_type_struct_array(
        #[case] example: &str,
        #[case] type_name: &str,
        #[case] encoded_type: &str,
    ) {
        let typed_data: TypedData<MailTypeStructArray> = load_typed_data(example);
        let res = typed_data.encode_type(type_name);
        assert_eq!(res, encoded_type);
    }

    #[rstest]
    #[case(
        TD,
        "StarkNetDomain",
        "0x1bfc207425a47a5dfa1a50a4f5241203f50624ca5fdf5e18755765416b8e288"
    )]
    #[case(
        TD,
        "Person",
        "0x2896dbe4b96a67110f454c01e5336edc5bbc3635537efd690f122f4809cc855"
    )]
    #[case(
        TD,
        "Mail",
        "0x13d89452df9512bf750f539ba3001b945576243288137ddb6c788457d4b2f79"
    )]
    #[case(
        TD_STRING,
        "String",
        "0x1933fe9de7e181d64298eecb44fc43b4cec344faa26968646761b7278df4ae2"
    )]
    #[case(
        TD_STRING,
        "Mail",
        "0x1ac6f84a5d41cee97febb378ddabbe1390d4e8036df8f89dee194e613411b09"
    )]
    #[case(
        TD_FELT_ARR,
        "Mail",
        "0x5b03497592c0d1fe2f3667b63099761714a895c7df96ec90a85d17bfc7a7a0"
    )]
    #[case(
        TD_STRUCT_ARR,
        "Post",
        "0x1d71e69bf476486b43cdcfaf5a85c00bb2d954c042b281040e513080388356d"
    )]
    #[case(
        TD_STRUCT_ARR,
        "Mail",
        "0x873b878e35e258fc99e3085d5aaad3a81a0c821f189c08b30def2cde55ff27"
    )]
    fn test_type_hash(
        #[case] example: &str,
        #[case] type_name: &str,
        #[case] expected_type_hash: &str,
    ) {
        match example {
            TD => {
                let typed_data: TypedData<MailType> = load_typed_data(example);
                let result = typed_data.type_hash(type_name);
                assert_eq!(format!("{:#x}", result), expected_type_hash);
            }
            TD_STRING => {
                let typed_data: TypedData<MailTypeLongString> = load_typed_data(example);
                let result = typed_data.type_hash(type_name);
                assert_eq!(format!("{:#x}", result), expected_type_hash);
            }
            TD_FELT_ARR => {
                let typed_data: TypedData<MailTypeFeltArray> = load_typed_data(example);
                let result = typed_data.type_hash(type_name);
                assert_eq!(format!("{:#x}", result), expected_type_hash);
            }
            TD_STRUCT_ARR => {
                let typed_data: TypedData<MailTypeStructArray> = load_typed_data(example);
                let result = typed_data.type_hash(type_name);
                assert_eq!(format!("{:#x}", result), expected_type_hash);
            }
            _ => panic!("Unknown example type"),
        }
    }

    #[test]
    fn test_struct_hash_starknet_domain() {
        let typed_data: TypedData<MailType> = load_typed_data(TD);
        let json_str = serde_json::to_string(&typed_data.domain).unwrap();
        let json_map: Map<String, Value> = serde_json::from_str(&json_str).unwrap();
        let result = typed_data.struct_hash("StarkNetDomain", &json_map);
        assert_eq!(
            format!("{:#x}", result),
            "0x54833b121883a3e3aebff48ec08a962f5742e5f7b973469c1f8f4f55d470b07"
        );
    }

    #[test]
    fn test_struct_hash_mail_message() {
        let typed_data: TypedData<MailType> = load_typed_data(TD);
        let json_str = serde_json::to_string(&typed_data.message).unwrap();
        let json_map: Map<String, Value> = serde_json::from_str(&json_str).unwrap();
        let result = typed_data.struct_hash("Mail", &json_map);
        assert_eq!(
            format!("{:#x}", result),
            "0x4758f1ed5e7503120c228cbcaba626f61514559e9ef5ed653b0b885e0f38aec"
        );
    }

    #[test]
    fn test_struct_hash_mail_long_string() {
        let typed_data: TypedData<MailTypeLongString> = load_typed_data(TD_STRING);
        let json_str = serde_json::to_string(&typed_data.message).unwrap();
        let json_map: Map<String, Value> = serde_json::from_str(&json_str).unwrap();
        let result = typed_data.struct_hash("Mail", &json_map);
        assert_eq!(
            format!("{:#x}", result),
            "0x1d16b9b96f7cb7a55950b26cc8e01daa465f78938c47a09d5a066ca58f9936f"
        );
    }

    #[test]
    fn test_struct_hash_mail_felt_array() {
        let typed_data: TypedData<MailTypeFeltArray> = load_typed_data(TD_FELT_ARR);
        let json_str = serde_json::to_string(&typed_data.message).unwrap();
        let json_map: Map<String, Value> = serde_json::from_str(&json_str).unwrap();
        let result = typed_data.struct_hash("Mail", &json_map);
        assert_eq!(
            format!("{:#x}", result),
            "0x26186b02dddb59bf12114f771971b818f48fad83c373534abebaaa39b63a7ce"
        );
    }

    #[test]
    fn test_struct_hash_mail_struct_array() {
        let typed_data: TypedData<MailTypeStructArray> = load_typed_data(TD_STRUCT_ARR);
        let json_str = serde_json::to_string(&typed_data.message).unwrap();
        let json_map: Map<String, Value> = serde_json::from_str(&json_str).unwrap();
        let result = typed_data.struct_hash("Mail", &json_map);
        assert_eq!(
            format!("{:#x}", result),
            "0x5650ec45a42c4776a182159b9d33118a46860a6e6639bb8166ff71f3c41eaef"
        );
    }

    #[rstest]
    #[case(
        TD,
        "0xcd2a3d9f938e13cd947ec05abc7fe734df8dd826",
        "0x6fcff244f63e38b9d88b9e3378d44757710d1b244282b435cb472053c8d78d0"
    )]
    #[case(
        TD_STRING,
        "0xcd2a3d9f938e13cd947ec05abc7fe734df8dd826",
        "0x691b977ee0ee645647336f01d724274731f544ad0d626b078033d2541ee641d"
    )]
    #[case(
        TD_FELT_ARR,
        "0xcd2a3d9f938e13cd947ec05abc7fe734df8dd826",
        "0x30ab43ef724b08c3b0a9bbe425e47c6173470be75d1d4c55fd5bf9309896bce"
    )]
    #[case(
        TD_STRUCT_ARR,
        "0xcd2a3d9f938e13cd947ec05abc7fe734df8dd826",
        "0x5914ed2764eca2e6a41eb037feefd3d2e33d9af6225a9e7fe31ac943ff712c"
    )]
    fn test_message_hash(
        #[case] example: &str,
        #[case] account_address: &str,
        #[case] msg_hash: &str,
    ) {
        let account_address = FieldElement::from_hex_be(account_address).unwrap();

        let result = match example {
            TD => {
                let typed_data: TypedData<MailType> = load_typed_data(example);
                typed_data.message_hash(account_address)
            }
            TD_STRING => {
                let typed_data: TypedData<MailTypeLongString> = load_typed_data(example);
                typed_data.message_hash(account_address)
            }
            TD_FELT_ARR => {
                let typed_data: TypedData<MailTypeFeltArray> = load_typed_data(example);
                typed_data.message_hash(account_address)
            }
            TD_STRUCT_ARR => {
                let typed_data: TypedData<MailTypeStructArray> = load_typed_data(example);
                typed_data.message_hash(account_address)
            }
            _ => panic!("Unsupported example type"),
        };

        assert_eq!(format!("{:#x}", result), msg_hash);
    }
}
