use std::char;
use std::collections::BTreeMap;
use std::cmp::min;
use std::io::{self, Read};

mod tables;
use tables::{STATES, GOTOS, CATCODE};

#[derive(Debug)]
pub enum JsonError {
    Truncated,
    NoObjects,
    MultipleObjects,
    Syntax,
    InvalidEscape(String),
    IO(io::Error),
}

#[derive(Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    fn into_string(self) -> String {
        match self {
            Value::String(s) => s,
            _ => panic!("wrong type - expected String, got {:?}", self),
        }
    }

    fn as_list(&mut self) -> &mut Vec<Value> {
        match self {
            &mut Value::List(ref mut l) => l,
            _ => panic!("wrong type - expected List, got {:?}", self),
        }
    }

    fn as_object(&mut self) -> &mut BTreeMap<String, Value> {
        match self {
            &mut Value::Object(ref mut o) => o,
            _ => panic!("wrong type - expected Object, got {:?}", self),
        }
    }
}

pub fn parse<R: Read>(input: R) -> Result<Value, JsonError> {
    let mut stack = vec![];
    let mut state = 0;
    let mut ds: Vec<Value> = vec![];    // data stack
    let mut ss = String::new();         // string stack
    let mut es = String::new();         // escape stack
    for maybe_ch in input.bytes() {
        let ch = try!(maybe_ch.map_err(JsonError::IO));
        let cat = CATCODE[min(ch, 0x7e) as usize];
        state = try!(parse_ch(cat, ch, &mut stack, state, &mut ds,
                              &mut ss, &mut es));
    }
    state = try!(parse_ch(CATCODE[32], '?' as u8, &mut stack, state,
                          &mut ds, &mut ss, &mut es));
    if state != 0 {
        return Err(JsonError::Truncated);
    }
    match ds.len() {
        0 => Err(JsonError::NoObjects),
        1 => Ok(ds.pop().unwrap()),
        _ => Err(JsonError::MultipleObjects),
    }
}

fn parse_ch(cat: u8, ch: u8, stack: &mut Vec<u8>, mut state: u8,
            ds: &mut Vec<Value>, ss: &mut String, es: &mut String)
        -> Result<u8, JsonError> {
    loop {
        let mut code: u16 = STATES[state as usize][cat as usize];
        let mut action: u8 = (code >> 8 & 0xFF) as u8;
        code = code & 0xFF;
        if action == 0xFF && code == 0xFF {
            return Err(JsonError::Syntax);
        } else if action >= 0x80 {
            stack.push(GOTOS[state as usize]);
            action -= 0x80;
        }
        if action > 0 {
            try!(do_action(action, ch, ds, ss, es));
        }
        if code == 0xFF {
            state = stack.pop().unwrap();
        } else {
            state = code as u8;
            return Ok(state);
        }
    }
}

fn do_action(action: u8, ch: u8, ds: &mut Vec<Value>, ss: &mut String,
             es: &mut String) -> Result<(), JsonError> {
    match action {
        0x1 => { // push list
            ds.push(Value::List(vec![]));
        },
        0x2 => { // push object
            ds.push(Value::Object(BTreeMap::new()));
        },
        0x3 => { // pop & append
            let v = ds.pop().unwrap();
            ds.last_mut().unwrap().as_list().push(v);
        },
        0x4 => { // pop pop & setitem
            let v = ds.pop().unwrap();
            let k = ds.pop().unwrap();
            ds.last_mut().unwrap().as_object().insert(k.into_string(), v);
        },
        0x5 => { // push null
            ds.push(Value::Null);
        },
        0x6 => { // push true
            ds.push(Value::Bool(true));
        },
        0x7 => { // push false
            ds.push(Value::Bool(false));
        },
        0x8 => { // push string
            ds.push(Value::String(ss.clone()));
            ss.clear();
            es.clear();
        },
        0x9 => { // push int
            ds.push(Value::Int(ss.parse().unwrap()));
            ss.clear();
        },
        0xA => { // push float
            ds.push(Value::Float(ss.parse().unwrap()));
            ss.clear();
        },
        0xB => { // push ch to ss
            ss.push(ch as char);
        },
        0xC => { // push ch to es
            es.push(ch as char);
        }
        0xD => { // push escape
            let c: u8 = match ch as char {
                'b' => 8,
                't' => 9,
                'n' => 10,
                'f' => 12,
                'r' => 13,
                _ => { return Err(JsonError::InvalidEscape(format!("\\{}", ch))); },
            };
            ss.push(c as char);
            es.clear();
        },
        0xE => { // push unicode code point
            let n = try!(u16::from_str_radix(es, 16).map_err(|_|
                    JsonError::InvalidEscape(format!("\\u{}", es))));
            if let Some(u) = char::from_u32(n as u32) {
                ss.push(u);
            } else {
                return Err(JsonError::InvalidEscape(format!("\\u{}", es)));
            }
            es.clear();
        },
        _ => panic!("JSON decoder bug"),
    }
    Ok(())
}
