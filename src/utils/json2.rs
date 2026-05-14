use crate::utils::json2::DataType::{Array, Boolean, Float, Int, Null, Object};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::ops::Range;

fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

pub(crate) struct JsonParser<'a> {
    input: &'a str,
    position: usize,
    index: OnceCell<HashMap<&'a str, Range<usize>>>,
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

    pub fn build_index(&mut self) -> Result<HashMap<&'a str, Range<usize>>, &'static str> {
        let mut index = HashMap::new();

        loop {
            self.skip_white_spaces();
            let key = self.read_key()?;
            self.skip_white_spaces();

            self.expect_byte(b':')?;

            let value_range = self.skip_value()?;

            index.insert(key, value_range);

            match self.current_byte() {
                Some(b',') => self.advance(),
                Some(b'}') => {
                    break;
                }
                Some(byte) if is_whitespace(byte) => {
                    self.skip_white_spaces();
                }
                _ => return Err("Invalid JSON format."),
            }
        }

        Ok(index)
    }

    fn skip_value(&mut self) -> Result<Range<usize>, &'static str> {
        match self.current() {
            b'"' => self.skip_string()?,
            b'{' => self.skip_object()?,
            b'[' => self.skip_array()?,
            b't' => self.expect_literal("true")?,
            b'f' => self.expect_literal("false")?,
            b'n' => self.expect_literal("null")?,
            b'-' | b'0'..=b'9' => self.skip_number()?,
            _ => Err("invalid json value"),
        }
    }
    fn skip_string(&mut self) -> Result<Range<usize>, &'static str> {
        self.advance(); // skip '"'
        let start = self.position;

        while !self.is_end() && self.current_byte() != Some(b'"') {
            self.advance();
        }

        self.expect_byte(b'"')?;

        let end = self.position;
        self.advance();
        Ok(start..end)
    }

    fn skip_object(&mut self) -> Result<Range<usize>, &'static str> {
        let start = self.position;

        self.advance();
        loop {
            self.skip_white_spaces();

            match self.current_byte() {
                Some(b'}') => {
                    self.advance();
                    return Ok(start..self.position);
                }

                Some(b'"') => {
                    self.skip_string()?; // key

                    self.skip_white_spaces();

                    self.expect_byte(b':')?;

                    self.advance();

                    self.skip_white_spaces();

                    self.skip_value()?;

                    self.skip_white_spaces();

                    match self.current() {
                        b',' => {
                            self.advance();
                        }
                        b'}' => {
                            self.advance();
                            return Ok(start..self.position);
                        }
                        _ => return Err("Expected ',' or '}' in object"),
                    }
                }

                _ => return Err("Expected object key or '}'"),
            }
        }
    }

    fn skip_number(&mut self) -> Result<Range<usize>, &'static str> {
        let start = self.position;
        let mut dot = false;
        self.advance();
        loop {
            match self.current_byte() {
                Some(b'0'..=b'9') => {
                    self.advance();
                }
                Some(b'.') if !dot => {
                    self.advance();
                    dot = true;
                }
                Some(b',') => {
                    let end = self.position;
                    self.advance();
                    return Ok(start..end);
                }
                Some(byte) if is_whitespace(byte) => {
                    self.advance();
                }
                _ => return Err("Expected number"),
            }
        }
    }

    fn read_key(&mut self) -> Result<&'a str, &'static str> {
        match self.current_byte() {
            Some(b'"') => self.read_string(),
            _ => Err("Invalid json value"),
        }
    }

    fn read_string(&mut self) -> Result<&'a str, &'static str> {
        let range = self.skip_string()?;
        Ok(std::str::from_utf8(&self.data[range.start..range.end]).unwrap())
    }

    fn current_byte(&self) -> Option<u8> {
        self.data.get(self.position).copied()
    }

    fn is_end(&self) -> bool {
        self.position >= self.len
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_white_spaces(&mut self) {
        while !self.is_end() && is_whitespace(self.current_byte().unwrap()) {
            self.position += 1
        }
    }

    fn expect_byte(&self, b: u8) -> Result<(), &'static str> {
        if !self.is_end() && self.current_byte() == Some(b) {
            return Ok(());
        }
        Err("Expected byte didnt found.")
    }

    fn expect_literal(&self, str: &str) -> Result<(), &'static str> {
        let start = self.position;
        let end = start + str.len();
        if std::str::from_utf8(&self.data[start..end])? == str {
            return Ok(());
        }
        Err("Expected literal didnt found.")
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
            DataType::String(data) => Ok(data),
            _ => Err("this is not a string"),
        }
    }

    pub(crate) fn unwrap_as_float(&self) -> Result<f64, &str> {
        match self {
            Float(data) => Ok(*data),
            _ => Err("this is not a Float"),
        }
    }

    pub(crate) fn unwrap_as_int(&self) -> Result<i32, &str> {
        match self {
            Int(data) => Ok(*data),
            _ => Err("this is not an Int"),
        }
    }

    pub(crate) fn unwrap_as_array(&self) -> Result<&Vec<DataType<'a>>, &str> {
        match self {
            Array(data) => Ok(data),
            _ => Err("this is not an Array"),
        }
    }

    pub(crate) fn unwrap_as_boolean(&self) -> Result<bool, &str> {
        match self {
            Boolean(data) => Ok(*data),
            _ => Err("this is not a Boolean"),
        }
    }

    pub(crate) fn unwrap_as_object(&self) -> Result<&HashMap<&'a str, DataType<'a>>, &str> {
        match self {
            Object(data) => Ok(data),
            _ => Err("this is not an Object"),
        }
    }

    pub(crate) fn is_null(&self) -> bool {
        match self {
            Null => true,
            _ => false,
        }
    }
}

impl<'a> JsonParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            index: OnceCell::new(),
        }
    }

    fn get(&mut self, key: &str) -> Result<Option<DataType<'a>>, &'static str> {
        match self.index.get_or_init(|| self.init_index()?).get(key) {
            None => Ok(None),
            Some(range) => self.parse(range)?,
        }
    }

    fn init_index(&mut self) -> Result<HashMap<&'a str, Range<usize>>, &'static str> {
        let mut scanner = JsonScanner::new(&self.input);
        scanner.build_index()
    }

    fn parse(&mut self, range: Range<usize>) -> Result<Option<DataType<'a>>, &'static str> {
        self.position = range.start;
        self.skip_white_spaces();
        let result = match self.current_byte() {
            Some(b'{') => self.parse_object(),
            Some(b'"') => Ok(Some(DataType::String(self.parse_string()?))),
            Some(b'[') => self.parse_array(),
            Some(b'+') | Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(b't') | Some(b'f') => self.parse_boolean(),
            Some(b'n') => Ok(self.parse_null()),
            _ => return Err("Unknown token detected."),
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
                _ => return Err("array parse failed"),
            }
        }
        Ok(Some(Array(array)))
    }

    fn parse_null(&mut self) -> DataType<'a> {
        self.position += 4;
        Null
    }

    fn parse_boolean(&mut self) -> Result<DataType<'a>, &'static str> {
        self.skip_white_spaces();
        if let Ok(boolean_str) = std::str::from_utf8(&self.data[self.position..(self.position + 4)])
        {
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

    fn current_byte(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    fn consume_token(&mut self) {
        self.position += 1;
    }

    fn skip_white_spaces(&mut self) {
        while !self.is_end() && is_whitespace(self.current_token()) {
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
}

pub(crate) trait JsonSerializable {
    fn serialize(&self, serializer: Serializer) -> String;
}

impl JsonSerializable for String {
    fn serialize(&self, serializer: Serializer) -> String {
        serializer.serialize_string(&self[..])
    }
}

impl JsonSerializable for f64 {
    fn serialize(&self, serializer: Serializer) -> String {
        serializer.serialize_f64(*self)
    }
}

impl JsonSerializable for i32 {
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
    fn new(key: String, value: &'a T) -> JsonEntry<'a, T> {
        JsonEntry { key, value }
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

    pub fn serialize_struct(&self) -> SerializerStruct {
        SerializerStruct::new()
    }

    pub fn serialize_seq(&self) -> SerializerSeq {
        SerializerSeq::new()
    }
}

pub(crate) struct SerializerStruct {
    fields: String,
}

impl SerializerStruct {
    fn new() -> SerializerStruct {
        SerializerStruct {
            fields: String::from("{"),
        }
    }

    pub fn serialize_field<T>(&mut self, name: &str, value: &T)
    where
        T: JsonSerializable,
    {
        self.fields.push_str("\"");
        self.fields.push_str(name);
        self.fields.push_str("\": ");
        self.fields
            .push_str(value.serialize(Serializer {}).as_str());
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

pub(crate) struct SerializerSeq {
    seq: String,
}

impl SerializerSeq {
    fn new() -> SerializerSeq {
        SerializerSeq {
            seq: String::from("["),
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
