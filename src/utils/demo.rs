use std::collections::HashMap;
use crate::utils::json::{JsonSerializable, Serializer};

struct Test {
    name: String,
    value: HashMap<String, Vec<String>>
}

impl JsonSerializable for Test {
    fn serialize(&self, serializer: Serializer) -> String {
        let mut serializer_struct = serializer.serialize_struct();
        serializer_struct.serialize_field("name", &self.name);
        serializer_struct.serialize_field("value", &self.value);
        serializer_struct.end()
    }
}

struct User {
    name: String,
    age: i32,
    test: Test
}

impl JsonSerializable for User {
    fn serialize(&self, serializer: Serializer) -> String {
        let mut serializer_struct = serializer.serialize_struct();
        serializer_struct.serialize_field("name", &self.name);
        serializer_struct.serialize_field("age", &self.age);
        serializer_struct.serialize_field("test", &self.test);
        serializer_struct.end()
    }
}

fn main() {
    let serializer = Serializer::new();
    let mut map = HashMap::new();
    map.insert(String::from("123123"), vec!(String::from("111"), String::from("222")));
    let t = Test {
        name: String::from("111"),
        value: map
    };

    let u = User {
        name: String::from("111"),
        age: 10,
        test: t
    };

    let string = u.serialize(serializer);
    println!("{}", string)
}
