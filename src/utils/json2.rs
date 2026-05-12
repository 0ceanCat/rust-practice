use std::collections::HashMap;
use crate::utils::json2::DataType::{Array, Boolean, Float, Int, Null, Object};

pub(crate) struct JsonParser<'a> {
    input: &'a str,
    scanned: bool,
    index: HashMap<&'a str, LazyJsonValue<'a>>,
}


struct JsonInput<'a> {
    data: &'a [u8],
    len: usize,
    position: usize,
}


impl<'a> JsonInput<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            data: input.as_bytes(),
            len: input.len(),
            position: 0,
        }
    }
}

struct JsonScanner<'a> {
    data: &'a [u8],
    len: usize,
    position: usize,
}

impl<'a> JsonScanner<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            data: input.as_bytes(),
            len: input.len(),
            position: 0,
        }
    }
}

#[derive(Debug)]
pub enum DataType<'a> {
    String(&'a str),
    Float(f64),
    Int(i32),
    Array(Vec<DataType<'a>>),
    Boolean(bool),
    Object(HashMap<&'a str, DataType<'a>>),
    Null,
}

impl<'a> DataType<'a> {
    pub(crate) fn unwrap_as_string(&self) -> Result<&str, &str> {
        match self {
            DataType::String(data) => { Ok(data) }
            _ => Err("this is not a string")
        }
    }

    pub(crate) fn unwrap_as_float(&self) -> Result<f64, &str> {
        match self {
            Float(data) => { Ok(*data) }
            _ => Err("this is not a Float")
        }
    }

    pub(crate) fn unwrap_as_int(&self) -> Result<i32, &str> {
        match self {
            Int(data) => { Ok(*data) }
            _ => Err("this is not an Int")
        }
    }

    pub(crate) fn unwrap_as_array(&self) -> Result<&Vec<DataType<'a>>, &str> {
        match self {
            Array(data) => { Ok(data) }
            _ => Err("this is not an Array")
        }
    }

    pub(crate) fn unwrap_as_boolean(&self) -> Result<bool, &str> {
        match self {
            Boolean(data) => { Ok(*data) }
            _ => Err("this is not a Boolean")
        }
    }

    pub(crate) fn unwrap_as_object(&self) -> Result<&HashMap<&'a str, DataType<'a>>, &str> {
        match self {
            Object(data) => { Ok(data) }
            _ => Err("this is not an Object")
        }
    }

    pub(crate) fn is_null(&self) -> bool {
        match self {
            Null => true,
            _ => false
        }
    }
}

impl<'a> JsonParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: JsonInput::new(input),
            scanned: false,
            index: HashMap::new(),
        }
    }

    fn get(&mut self, key: &str) -> Result<DataType<'a>, &'static str> {
        if !self.scanned {
            self.scanned = true;
            self.parse()?;
        }
    }

    fn parse(&mut self) -> Result<DataType<'a>, &'static str> {
        self.skip_white_spaces();
        let result = match self.current_token() {
            b'{' => self.parse_object(),
            b'"' => Ok(DataType::String(self.parse_string()?)),
            b'[' => self.parse_array(),
            b'+' | b'-' | b'0'..=b'9' => self.parse_number(),
            b't' | b'f' => self.parse_boolean(),
            b'n' => Ok(self.parse_null()),
            _ => { return Err("Unknown token detected.") }
        };
        self.skip_white_spaces();
        result
    }

    fn parse_object(&mut self) -> Result<DataType<'a>, &'static str> {
        self.consume_token(); // skip '{'
        let mut result: HashMap<&'a str, DataType<'a>> = HashMap::new();

        while !self.is_end() && self.current_token() != b'}' {
            let key = self.parse_string()?;
            if self.current_token() != b':' {
                return Err("':' is expected");
            }
            self.consume_token(); // skip ':'
            let value = self.parse()?;
            result.insert(key, value);
            if !self.is_end() && self.current_token() == b',' {
                self.consume_token();
            } else if !self.is_end() && self.current_token() != b'}' {
                return Err("object parse failed");
            }
        }

        self.consume_token();
        Ok(Object(result))
    }

    fn parse_string(&mut self) -> Result<&'a str, &'static str> {
        self.skip_white_spaces();
        self.consume_token(); // skip '"'
        let start = self.position;

        while !self.is_end() {
            if self.current_token() != b'"' {
                self.consume_token();
            } else {
                let end = self.position;
                self.consume_token(); // skip '"'
                self.skip_white_spaces();
                let str = std::str::from_utf8(&self.data[start..end])
                    .map_err(|_| "String parse failed")?;

                return Ok(str);
            }
        }
        Err("String parse failed")
    }

    fn parse_array(&mut self) -> Result<DataType<'a>, &'static str> {
        self.consume_token(); // skip '['
        let mut array: Vec<DataType<'a>> = Vec::new();

        while !self.is_end() {
            array.push(self.parse()?);
            let current = self.current_token();
            self.consume_token();
            match current {
                b']' => break,
                b',' => continue,
                _ => { return Err("array parse failed") }
            }
        }
        Ok(Array(array))
    }

    fn parse_null(&mut self) -> DataType<'a> {
        self.position += 4;
        Null
    }

    fn parse_boolean(&mut self) -> Result<DataType<'a>, &'static str> {
        self.skip_white_spaces();
        if let Ok(boolean_str) = std::str::from_utf8(&self.data[self.position..(self.position + 4)]) {
            self.position += 4;

            let result: bool;

            if boolean_str == "true" {
                result = true
            } else if boolean_str == "fals" && self.current_token() == b'e' {
                result = false;
                self.consume_token(); // skip 'e'
            } else {
                return Err("Boolean parse failed");
            }
            return Ok(Boolean(result));
        }

        Err("Boolean parse failed")
    }

    fn parse_number(&mut self) -> Result<DataType<'a>, &'static str> {
        let negative = self.current_token() == b'-';
        let sign: i32 = if negative { -1 } else { 1 };
        if negative || self.current_token() == b'+' {
            self.consume_token();
        }

        let first_part = self.parse_int()?;

        if !self.is_end() && self.current_token() == b'.' {
            self.consume_token();

            let fraction = self.parse_fraction()?;
            let value = first_part as f64 + fraction;

            Ok(Float(sign as f64 * value))
        } else {
            Ok(Int(sign * first_part))
        }
    }

    fn parse_int(&mut self) -> Result<i32, &'static str> {
        let mut result = 0;
        let mut has_digit = false;

        while !self.is_end() {
            match self.current_token() {
                b'0'..=b'9' => {
                    has_digit = true;
                    result = result * 10 + JsonParser::byte_to_integer(self.current_token());
                    self.consume_token();
                }
                _ => break,
            }
        }

        if !has_digit {
            return Err("expected digit");
        }

        Ok(result)
    }

    fn parse_fraction(&mut self) -> Result<f64, &'static str> {
        let mut result = 0.0;
        let mut base = 10.0;
        let mut has_digit = false;

        while !self.is_end() {
            match self.current_token() {
                b'0'..=b'9' => {
                    has_digit = true;
                    result += JsonParser::byte_to_float(self.current_token()) / base;
                    base *= 10.0;
                    self.consume_token();
                }
                _ => break,
            }
        }

        if !has_digit {
            return Err("expected digit after decimal point");
        }

        Ok(result)
    }

    fn current_token(&self) -> u8 {
        self.data[self.position]
    }

    fn consume_token(&mut self) {
        self.position += 1;
    }

    fn skip_white_spaces(&mut self) {
        while !self.is_end() && JsonParser::is_whitespace(self.current_token()) {
            self.position += 1
        }
    }

    fn is_end(&self) -> bool {
        self.position >= self.len
    }

    fn byte_to_integer(byte: u8) -> i32 {
        (byte - b'0') as i32
    }

    fn byte_to_float(byte: u8) -> f64 {
        (byte - b'0') as f64
    }

    fn is_whitespace(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | b'\r')
    }
}

pub(crate) trait JsonSerializable {
    fn serialize(&self, serializer: Serializer) -> String;
}

impl JsonSerializable for String {
    fn serialize(&self, serializer: Serializer) -> String {
        serializer.serialize_string(&self[..])
    }
}

impl JsonSerializable for f64
{
    fn serialize(&self, serializer: Serializer) -> String {
        serializer.serialize_f64(*self)
    }
}

impl JsonSerializable for i32
{
    fn serialize(&self, serializer: Serializer) -> String {
        serializer.serialize_i32(*self)
    }
}

impl<T> JsonSerializable for Vec<T>
where
    T: JsonSerializable,
{
    fn serialize(&self, serializer: Serializer) -> String {
        let mut seq = serializer.serialize_seq();
        for e in self {
            seq.serialize_element(e);
        }
        seq.end()
    }
}

impl<T> JsonSerializable for HashMap<String, T>
where
    T: JsonSerializable,
{
    fn serialize(&self, serializer: Serializer) -> String {
        let mut seq = serializer.serialize_struct();
        for e in self {
            seq.serialize_field(e.0, e.1);
        }
        seq.end()
    }
}

struct JsonEntry<'a, T>
where
    T: JsonSerializable,
{
    key: String,
    value: &'a T,
}

impl<'a, T> JsonEntry<'a, T>
where
    T: JsonSerializable,
{
    fn new(key: String, value: &'a T) -> JsonEntry<'a, T>
    {
        JsonEntry {
            key,
            value,
        }
    }
}

pub(crate) struct Serializer {}

impl Serializer {
    pub fn new() -> Serializer {
        Serializer {}
    }
    pub fn serialize_string(&self, str: &str) -> String {
        format!("\"{str}\"")
    }

    pub fn serialize_bool(&self, b: bool) -> String {
        b.to_string()
    }

    pub fn serialize_i32(&self, i: i32) -> String {
        i.to_string()
    }

    pub fn serialize_f64(&self, f: f64) -> String {
        f.to_string()
    }

    pub fn serialize_struct(&self) -> SerializerStruct
    {
        SerializerStruct::new()
    }

    pub fn serialize_seq(&self) -> SerializerSeq
    {
        SerializerSeq::new()
    }
}

pub(crate) struct SerializerStruct
{
    fields: String,
}

impl SerializerStruct
{
    fn new() -> SerializerStruct {
        SerializerStruct {
            fields: String::from("{")
        }
    }

    pub fn serialize_field<T>(&mut self, name: &str, value: &T)
    where
        T: JsonSerializable,
    {
        self.fields.push_str("\"");
        self.fields.push_str(name);
        self.fields.push_str("\": ");
        self.fields.push_str(value.serialize(Serializer {}).as_str());
        self.fields.push(',');
    }

    pub fn end(mut self) -> String {
        if self.fields.len() > 1 {
            self.fields.remove(self.fields.len() - 1);
        }
        self.fields.push('}');
        self.fields
    }
}

pub(crate) struct SerializerSeq
{
    seq: String,
}

impl SerializerSeq
{
    fn new() -> SerializerSeq {
        SerializerSeq {
            seq: String::from("[")
        }
    }

    fn serialize_element<T>(&mut self, elem: &T)
    where
        T: JsonSerializable,
    {
        self.seq.push_str(elem.serialize(Serializer {}).as_str());
        self.seq.push(',');
    }

    fn end(mut self) -> String {
        if self.seq.len() > 1 {
            self.seq.remove(self.seq.len() - 1);
        }
        self.seq.push(']');
        self.seq
    }
}