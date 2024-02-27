use std::collections::HashMap;
use crate::utils::json::DataType::{Array, Boolean, Float, Int, Null, Object};

pub(crate) struct JsonParser {
    data: Vec<char>,
    len: usize,
    position: usize,
}

pub enum DataType {
    String(String),
    Float(f64),
    Int(i32),
    Array(Vec<DataType>),
    Boolean(bool),
    Object(HashMap<String, DataType>),
    Null,
}

impl JsonParser {
    pub fn new(str: &str) -> Self {
        let chars: Vec<char> = str.chars().collect();
        let len = chars.len();
        JsonParser {
            data: chars,
            len,
            position: 0,
        }
    }

    pub fn parse_to_map(mut self) -> HashMap<String, DataType> {
        let Object(map) = self.parse_object() else {
            panic!("parse json failed")
        };
        map
    }

    fn parse(&mut self) -> DataType {
        self.skip_white_spaces();
        let result = match self.current_token() {
            '{' => self.parse_object(),
            '"' => self.parse_string(),
            '[' => self.parse_array(),
            '+' | '-' | '0'..='9' => self.parse_number(),
            't' | 'f' => self.parse_boolean(),
            'n' => self.parse_null(),
            _ => { panic!("wtf??") }
        };
        self.skip_white_spaces();
        result
    }

    fn parse_object(&mut self) -> DataType {
        self.consume_token(); // skip '{'
        let mut result: HashMap<String, DataType> = HashMap::new();
        while !self.is_end() && self.current_token() != '}' {
            if let DataType::String(key) = self.parse_string() {
                if self.current_token() != ':' {
                    panic!("':' is expected")
                }
                self.consume_token(); // skip ':'
                let value = self.parse();
                result.insert(key, value);
                if !self.is_end() && self.current_token() == ',' {
                    self.consume_token();
                }else if !self.is_end() && self.current_token() != '}' {
                    panic!("object parse failed");
                }
            }
        }
        self.consume_token();
        Object(result)
    }

    fn parse_string(&mut self) -> DataType {
        self.skip_white_spaces();
        self.consume_token(); // skip '"'
        let mut result = String::new();

        while !self.is_end() {
            if self.current_token() != '"' {
                result.push(self.current_token());
                self.consume_token();
            } else {
                self.consume_token(); // skip '"'
                self.skip_white_spaces();
                return DataType::String(result);
            }
        }
        panic!("string parse failed")
    }

    fn parse_array(&mut self) -> DataType {
        self.consume_token(); // skip '['
        let mut array: Vec<DataType> = Vec::new();

        while !self.is_end() {
            array.push(self.parse());
            let current = self.current_token();
            self.consume_token();
            match current {
                ']' => break,
                ',' => continue,
                _ => { panic!("array parse failed") }
            }
        }
        Array(array)
    }

    fn parse_null(&mut self) -> DataType {
        self.position += 4;
        Null
    }

    fn parse_boolean(&mut self) -> DataType {
        self.skip_white_spaces();
        let mut read_str = String::new();

        self.data[self.position..(self.position + 4)].iter().for_each(|c| read_str.push(*c));
        self.position += 4;

        let result: bool;

        if read_str == "true" {
            result = true
        } else if read_str == "fals" && self.current_token() == 'e' {
            result = false;
            self.consume_token(); // skip 'e'
        } else {
            panic!("boolean parse failed");
        }
        Boolean(result)
    }

    fn parse_number(&mut self) -> DataType {
        let negative = self.current_token() == '-';
        let sign: i32 = if negative { -1 } else { 1 };
        if negative || self.current_token() == '+' {
            self.consume_token();
        }
        let mut result: DataType = self.parse_int();
        if let Int(first_part) = result {
            if !self.is_end() && self.current_token() == '.' {
                self.consume_token();
                let mut base = 1.0;
                if let Int(nb) = self.parse_int() {
                    let second_part = nb as f64;
                    while second_part / base > 0.0 {
                        base *= 10.0
                    }
                    result = Float(sign as f64 * (first_part as f64 + second_part / base))
                }
            } else {
                result = Int(sign * first_part)
            }
        }
        result
    }

    fn parse_int(&mut self) -> DataType {
        match self.current_token() {
            '0'..='9' => {
                let mut result = 0;
                while !self.is_end() && ('0'..='9').contains(&self.current_token()) {
                    result = result * 10 + JsonParser::char_to_integer(self.current_token());
                    self.consume_token();
                }
                return Int(result);
            }
            _ => panic!("parse int failed")
        }
    }

    fn current_token(&self) -> char {
        return self.data[self.position];
    }

    fn consume_token(&mut self) {
        self.position += 1;
    }

    fn skip_white_spaces(&mut self) {
        let white_space = " \t\r\n";
        while !self.is_end() && white_space.contains(self.current_token()) {
            self.position += 1
        }
    }

    fn is_end(&self) -> bool {
        self.position >= self.len
    }

    fn char_to_integer(c: char) -> i32 {
        c as i32 - 0x30
    }
}