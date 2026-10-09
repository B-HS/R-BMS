//! The reference's scalar coercions, in one place.
//!
//! The reference runs skins on LuaJ and reads every Lua value through `LuaValue.toboolean`, `toint`,
//! `tolong`, `tofloat` and `tojstring`. Those are not Lua's own `tonumber` and `tostring`: none of
//! them ever raises, and a value of the wrong type reads as zero rather than as an error. Three
//! parts of this crate depend on reading values the same way -- the table converter, the results of
//! the functions a frame calls, and the arguments of `main_state` -- so the rules live here and
//! nowhere else.

use mlua::Value;

/// The prefix of a hexadecimal numeral.
const HEX_PREFIXES: [&str; 2] = ["0x", "0X"];

/// The radix of a hexadecimal numeral.
const HEX_RADIX: u32 = 16;

/// What `tojstring` makes of `nil`.
const NIL_TEXT: &str = "nil";

/// What `tojstring` makes of a number that is not one.
const NAN_TEXT: &str = "nan";

/// What `tojstring` makes of positive infinity.
const INFINITY_TEXT: &str = "inf";

/// What `tojstring` makes of negative infinity.
const NEGATIVE_INFINITY_TEXT: &str = "-inf";

/// The smallest magnitude Java prints a `float` at in plain decimal notation. A smaller one, other
/// than zero, is printed in scientific notation (`Float.toString`).
const JAVA_PLAIN_FLOAT_MIN: f32 = 1.0e-3;

/// The magnitude from which Java prints a `float` in scientific notation (`Float.toString`).
const JAVA_PLAIN_FLOAT_LIMIT: f32 = 1.0e7;

/// The fraction Java always prints after a `float` whose digits have none.
const JAVA_EMPTY_FRACTION: &str = ".0";

/// The exponent marker of Java's scientific notation.
const JAVA_EXPONENT_MARKER: char = 'E';

/// `LuaValue.toboolean`: only `nil` and `false` are false. The number zero is true.
pub fn to_boolean(value: &Value) -> bool {
    !matches!(value, Value::Nil | Value::Boolean(false))
}

/// The number a value reads as, or `None` when it is not a number and not a string that spells one.
///
/// A string is read the way Lua reads a numeral: surrounding blanks are ignored, and both decimal
/// and hexadecimal forms are accepted.
pub fn to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(number) => Some(*number as f64),
        Value::Number(number) => Some(*number),
        Value::String(text) => parse_numeral(text.to_string_lossy().trim()),
        _ => None,
    }
}

/// `LuaValue.tolong`: the number cast toward zero, saturating at the ends of the range, with NaN
/// and every non-number reading as zero.
pub fn to_long(value: &Value) -> i64 {
    match value {
        Value::Integer(number) => *number,
        other => to_number(other).map_or(0, |number| number as i64),
    }
}

/// `LuaValue.toint`: [`to_long`] narrowed the way Java narrows a `long` to an `int`, by keeping its
/// low 32 bits.
pub fn to_int(value: &Value) -> i32 {
    to_long(value) as i32
}

/// `LuaValue.tofloat`: the number narrowed to single precision, with every non-number reading as
/// zero.
pub fn to_float(value: &Value) -> f32 {
    to_number(value).map_or(0.0, |number| number as f32)
}

/// `LuaValue.tojstring`: a string as it is, a number as LuaJ prints one, `nil` and booleans by name.
///
/// A table, function or other reference prints as its type name, which is all a skin can usefully do
/// with one in a string field; LuaJ appends an address that means nothing outside its own process.
pub fn to_jstring(value: &Value) -> String {
    match value {
        Value::Nil => NIL_TEXT.to_owned(),
        Value::Boolean(flag) => flag.to_string(),
        Value::Integer(number) => number.to_string(),
        Value::Number(number) => number_text(*number),
        Value::String(text) => text.to_string_lossy(),
        other => other.type_name().to_owned(),
    }
}

/// `LuaDouble.tojstring`: a number with no fraction prints as the `long` it equals, and any other
/// prints as Java prints it narrowed to a `float`.
///
/// The narrowing is the reference's and is kept: a third prints as `0.33333334`, and a number a
/// `float` cannot tell from a whole one prints with a `.0` the whole one would not have.
fn number_text(number: f64) -> String {
    let whole = number as i64;
    if whole as f64 == number {
        return whole.to_string();
    }
    if number.is_nan() {
        return NAN_TEXT.to_owned();
    }
    if number.is_infinite() {
        return if number < 0.0 { NEGATIVE_INFINITY_TEXT } else { INFINITY_TEXT }.to_owned();
    }
    java_float_text(number as f32)
}

/// `Float.toString`: the fewest digits that tell the value from its neighbours, with at least one
/// on each side of the point, in plain notation from a thousandth up to ten million and in
/// scientific notation outside that.
fn java_float_text(number: f32) -> String {
    let magnitude = number.abs();
    if magnitude == 0.0 || (JAVA_PLAIN_FLOAT_MIN..JAVA_PLAIN_FLOAT_LIMIT).contains(&magnitude) {
        let plain = number.to_string();
        return if plain.contains('.') { plain } else { format!("{plain}{JAVA_EMPTY_FRACTION}") };
    }
    let scientific = format!("{number:E}");
    match scientific.split_once(JAVA_EXPONENT_MARKER) {
        Some((digits, exponent)) if !digits.contains('.') => format!("{digits}{JAVA_EMPTY_FRACTION}{JAVA_EXPONENT_MARKER}{exponent}"),
        _ => scientific,
    }
}

/// Reads one Lua numeral.
fn parse_numeral(text: &str) -> Option<f64> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    if let Some(hex) = HEX_PREFIXES.iter().find_map(|prefix| digits.strip_prefix(prefix)) {
        let magnitude = u64::from_str_radix(hex, HEX_RADIX).ok()? as f64;
        return Some(if negative { -magnitude } else { magnitude });
    }
    text.parse::<f64>().ok().filter(|_| digits.starts_with(|first: char| first.is_ascii_digit() || first == '.'))
}

#[cfg(test)]
mod tests {
    use mlua::{Lua, Value};

    use super::{to_boolean, to_float, to_int, to_jstring, to_long};

    fn eval(lua: &Lua, source: &str) -> Value {
        lua.load(source).eval().expect("the sample chunk runs")
    }

    #[test]
    fn zero_is_true_and_only_nil_and_false_are_false() {
        let lua = Lua::new();
        assert!(to_boolean(&eval(&lua, "return 0")));
        assert!(to_boolean(&eval(&lua, "return ''")));
        assert!(!to_boolean(&eval(&lua, "return nil")));
        assert!(!to_boolean(&eval(&lua, "return false")));
    }

    #[test]
    fn integers_truncate_toward_zero_and_non_numbers_read_as_zero() {
        let lua = Lua::new();
        assert_eq!(to_int(&eval(&lua, "return 1920 / 7")), 274);
        assert_eq!(to_int(&eval(&lua, "return -2.9")), -2);
        assert_eq!(to_int(&eval(&lua, "return '12'")), 12);
        assert_eq!(to_int(&eval(&lua, "return '0x10'")), 16);
        assert_eq!(to_int(&eval(&lua, "return 'twelve'")), 0);
        assert_eq!(to_int(&eval(&lua, "return {}")), 0);
        assert_eq!(to_int(&eval(&lua, "return true")), 0);
        assert_eq!(to_int(&eval(&lua, "return 0/0")), 0);
        assert_eq!(to_long(&eval(&lua, "return -2^63")), i64::MIN);
        assert_eq!(to_float(&eval(&lua, "return '1.5'")), 1.5);
    }

    #[test]
    fn a_whole_number_prints_without_a_fraction() {
        let lua = Lua::new();
        assert_eq!(to_jstring(&eval(&lua, "return 10 / 2")), "5");
        assert_eq!(to_jstring(&eval(&lua, "return 'src'")), "src");
        assert_eq!(to_jstring(&eval(&lua, "return nil")), "nil");
        assert_eq!(to_jstring(&eval(&lua, "return true")), "true");
        assert_eq!(to_jstring(&eval(&lua, "return -0.0")), "0");
        assert_eq!(to_jstring(&eval(&lua, "return 2^53")), "9007199254740992");
    }

    #[test]
    fn a_number_with_a_fraction_prints_as_java_prints_a_float() {
        let lua = Lua::new();
        assert_eq!(to_jstring(&eval(&lua, "return 5.5")), "5.5");
        assert_eq!(to_jstring(&eval(&lua, "return -0.25")), "-0.25");
        assert_eq!(to_jstring(&eval(&lua, "return 1 / 3")), "0.33333334");
        assert_eq!(to_jstring(&eval(&lua, "return 0.001")), "0.001");
        assert_eq!(to_jstring(&eval(&lua, "return 0.00015")), "1.5E-4");
        assert_eq!(to_jstring(&eval(&lua, "return 0.0001")), "1.0E-4");
        assert_eq!(to_jstring(&eval(&lua, "return 1234567.5")), "1234567.5");
        assert_eq!(to_jstring(&eval(&lua, "return 12345678.5")), "1.2345678E7");
        assert_eq!(to_jstring(&eval(&lua, "return 5.0000000001")), "5.0");
        assert_eq!(to_jstring(&eval(&lua, "return 1e-50")), "0.0");
        assert_eq!(to_jstring(&eval(&lua, "return 0/0")), "nan");
        assert_eq!(to_jstring(&eval(&lua, "return 1/0")), "inf");
        assert_eq!(to_jstring(&eval(&lua, "return -1/0")), "-inf");
    }
}
