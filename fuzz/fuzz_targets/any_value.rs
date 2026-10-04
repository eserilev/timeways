//! Any value that another addon can leave in a global of the game: a saved variable that
//! it wrote, or the table of a roleplay addon. Texts are bytes, so they can hold control
//! characters and text that is not UTF-8.

use arbitrary::Arbitrary;
use mlua::{Lua, Value};

/// Deeper tables become nil, so a value stays small.
const MAX_DEPTH: u8 = 4;

#[derive(Arbitrary, Debug)]
pub enum AnyValue {
    Integer(i32),
    Real(f64),
    Text(Vec<u8>),
    Bool(bool),
    Table(Vec<(Vec<u8>, AnyValue)>),
    List(Vec<AnyValue>),
}

impl AnyValue {
    pub fn to_lua(&self, lua: &Lua) -> Value {
        self.at_depth(lua, 0)
    }

    fn at_depth(&self, lua: &Lua, depth: u8) -> Value {
        match self {
            _ if depth > MAX_DEPTH => Value::Nil,
            AnyValue::Integer(number) => Value::Integer((*number).into()),
            AnyValue::Real(number) => Value::Number(*number),
            AnyValue::Text(bytes) => Value::String(lua.create_string(bytes).unwrap()),
            AnyValue::Bool(value) => Value::Boolean(*value),
            AnyValue::Table(fields) => {
                let table = lua.create_table().unwrap();
                for (key, value) in fields {
                    let key = lua.create_string(key).unwrap();
                    table.set(key, value.at_depth(lua, depth + 1)).unwrap();
                }
                Value::Table(table)
            }
            AnyValue::List(items) => {
                let items = items.iter().map(|item| item.at_depth(lua, depth + 1));
                Value::Table(lua.create_sequence_from(items).unwrap())
            }
        }
    }
}
