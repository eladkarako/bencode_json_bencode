//! Convert between JSON and bencode.
//!
//! JSON values are mapped to bencode as follows:
//!
//! - `null` is rejected because bencode has no null value.
//! - Booleans become integers: `false` becomes `0`, and `true` becomes `1`.
//! - Numbers must fit in a signed 64-bit integer.
//! - Strings become byte strings.
//! - Arrays become lists.
//! - Objects become dictionaries.
//!
//! Non-UTF-8 bencode byte strings are represented in JSON using this format:
//!
//! ```text
//! "[binary: ff00ab]"
//! ```

use is_terminal::IsTerminal;
use serde_json::{Map, Number, Value};
use std::{
    env, fs,
    io::{self, Read, Write},
    path::PathBuf,
};

/// A value in the bencode data format.
#[derive(Debug, Clone)]
enum Bencode {
    /// A signed integer encoded as `i<number>e`.
    Integer(i64),

    /// An arbitrary sequence of bytes.
    Bytes(Vec<u8>),

    /// An ordered collection of bencode values.
    List(Vec<Bencode>),

    /// An ordered collection of byte-string keys and bencode values.
    Dict(Vec<(Vec<u8>, Bencode)>),
}

/// Program entry point.
///
/// Errors are printed to standard error and result in a non-zero exit code.
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

/// Parses command-line arguments, reads input, performs the conversion, and
/// writes the result to standard output.
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os();

    // The first argument is the executable name.
    let program = args.next().unwrap_or_default();

    let first = args.next();
    let second = args.next();

    // This program accepts at most two arguments:
    //
    //     <mode> <file>
    //
    // A single argument can either be a mode or an input-file path.
    if args.next().is_some() {
        usage(&program);
        return Err("too many arguments".into());
    }

    let (mode, input_path): (Option<&'static str>, Option<PathBuf>) =
        match (first, second) {
            (None, None) => (None, None),

            (Some(first), None) => match first.to_str() {
                Some("json2bencode") => (Some("json2bencode"), None),
                Some("bencode2json") => (Some("bencode2json"), None),

                // If the only argument is not a recognized mode, treat it as
                // an input-file path.
                _ => (None, Some(PathBuf::from(first))),
            },

            (Some(first), Some(second)) => {
                let mode = match first.to_str() {
                    Some("json2bencode") => "json2bencode",
                    Some("bencode2json") => "bencode2json",

                    Some(other) => {
                        usage(&program);
                        return Err(format!(
                            "unknown mode: {other}"
                        )
                            .into());
                    }

                    None => {
                        usage(&program);
                        return Err("mode is not valid UTF-8".into());
                    }
                };

                (Some(mode), Some(PathBuf::from(second)))
            }

            // `args.next()` cannot return a second argument when the first
            // argument is missing.
            (None, Some(_)) => {
                unreachable!(
                    "the second argument cannot exist without a first argument"
                )
            }
        };

    let input_data = match input_path {
        Some(path) => {
            eprintln!("reading input file: {}", path.display());
            fs::read(path)?
        }

        None => {
            let stdin = io::stdin();

            // Avoid blocking when the program is run interactively without
            // redirected input.
            if stdin.is_terminal() {
                usage(&program);
                return Err(
                    "no input file was provided and stdin is interactive"
                        .into(),
                );
            }

            let mut data = Vec::new();
            stdin.lock().read_to_end(&mut data)?;

            if data.is_empty() {
                usage(&program);
                return Err(
                    "no input file was provided and stdin contained no bytes"
                        .into(),
                );
            }

            eprintln!("reading input from stdin");
            data
        }
    };

    let output_data = match mode {
        Some("json2bencode") => {
            eprintln!("conversion mode: JSON to bencode");
            json_to_bencode(&input_data)?
        }

        Some("bencode2json") => {
            eprintln!("conversion mode: bencode to JSON");
            bencode_to_json(&input_data)?
        }

        None => {
            eprintln!("conversion mode: automatic detection");
            sniff_and_convert(&input_data)?.1
        }

        _ => unreachable!(),
    };

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    stdout.write_all(&output_data)?;
    stdout.flush()?;

    Ok(())
}

/// Attempts to parse the input as bencode.
///
/// If parsing succeeds and consumes the entire input, the input is treated as
/// bencode. Otherwise, it is treated as JSON5.
fn sniff_and_convert(
    data: &[u8]
) -> Result<(&'static str, Vec<u8>), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(data);

    if let Ok(value) = parser.parse_value() {
        if parser.is_at_end() {
            eprintln!("detected input as bencode");

            return Ok((
                "json",
                bencode_to_json_value_bytes(&value)?,
            ));
        }
    }

    eprintln!("detected input as JSON");

    Ok(("bencode", json_to_bencode(data)?))
}

/// Converts a bencode value to pretty-printed JSON bytes.
fn bencode_to_json_value_bytes(
    value: &Bencode
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let json = bencode_to_json_value(value)?;

    let mut output = serde_json::to_vec_pretty(&json)?;
    output.push(b'\n');

    Ok(output)
}

/// Prints command-line usage information.
fn usage(program: &std::ffi::OsStr) {
    let program = program.to_string_lossy();

    eprintln!(
        "usage:\n\
         \n\
         {program} [json2bencode|bencode2json] <file>\n\
         {program} [json2bencode|bencode2json] < input"
    );
}

/// Converts JSON5 input into bencode bytes.
fn json_to_bencode(
    data: &[u8]
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let text = std::str::from_utf8(data)?;
    let json: Value = json5::from_str(text)?;
    let bencode = json_to_bencode_value(&json)?;

    let mut output = Vec::new();
    encode_bencode(&bencode, &mut output)?;

    Ok(output)
}

/// Converts a JSON value into its corresponding bencode representation.
fn json_to_bencode_value(
    value: &Value
) -> Result<Bencode, Box<dyn std::error::Error>> {
    match value {
        // Bencode does not define a null value.
        Value::Null => {
            Err("null is not representable in bencode".into())
        }

        // Represent booleans as the conventional integer values 0 and 1.
        Value::Bool(value) => {
            Ok(Bencode::Integer(if *value { 1 } else { 0 }))
        }

        // Bencode integers are signed 64-bit integers.
        Value::Number(number) => {
            let integer = number.as_i64().ok_or(
                "bencode only supports signed integer numbers",
            )?;

            Ok(Bencode::Integer(integer))
        }

        Value::String(string) => {
            // Strings using the special binary marker are decoded into raw
            // bytes instead of being encoded as UTF-8 text.
            if let Some(bytes) = parse_binary_marker(string)? {
                Ok(Bencode::Bytes(bytes))
            } else {
                Ok(Bencode::Bytes(string.as_bytes().to_vec()))
            }
        }

        Value::Array(values) => {
            let mut result = Vec::with_capacity(values.len());

            for value in values {
                result.push(json_to_bencode_value(value)?);
            }

            Ok(Bencode::List(result))
        }

        Value::Object(object) => {
            let mut result = Vec::with_capacity(object.len());

            for (key, value) in object {
                result.push((
                    key.as_bytes().to_vec(),
                    json_to_bencode_value(value)?,
                ));
            }

            Ok(Bencode::Dict(result))
        }
    }
}

/// Parses a string in the form `[binary: <hex>]`.
///
/// Returns `Ok(None)` when the input is an ordinary string.
fn parse_binary_marker(
    value: &str
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let Some(hex_text) = value
        .strip_prefix("[binary:")
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Ok(None);
    };

    let hex_text = hex_text.trim();

    if hex_text.len() % 2 != 0 {
        return Err(format!(
            "binary marker has an odd number of hex digits: {value}"
        )
            .into());
    }

    let bytes = hex::decode(hex_text).map_err(|error| {
        format!("invalid binary marker {value:?}: {error}")
    })?;

    Ok(Some(bytes))
}

/// Encodes a bencode value into the supplied output buffer.
fn encode_bencode(
    value: &Bencode,
    output: &mut Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    match value {
        Bencode::Integer(value) => {
            output.extend_from_slice(b"i");
            output.extend_from_slice(value.to_string().as_bytes());
            output.extend_from_slice(b"e");
        }

        Bencode::Bytes(bytes) => {
            // A byte string is encoded as:
            //
            //     <length>:<bytes>
            //
            output.extend_from_slice(
                bytes.len().to_string().as_bytes(),
            );
            output.extend_from_slice(b":");
            output.extend_from_slice(bytes);
        }

        Bencode::List(values) => {
            output.extend_from_slice(b"l");

            for value in values {
                encode_bencode(value, output)?;
            }

            output.extend_from_slice(b"e");
        }

        Bencode::Dict(entries) => {
            output.extend_from_slice(b"d");

            for (key, value) in entries {
                // Dictionary keys are encoded as bencode byte strings.
                encode_bencode(
                    &Bencode::Bytes(key.clone()),
                    output,
                )?;
                encode_bencode(value, output)?;
            }

            output.extend_from_slice(b"e");
        }
    }

    Ok(())
}

/// Converts bencode input into pretty-printed JSON bytes.
fn bencode_to_json(
    data: &[u8]
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut parser = Parser::new(data);
    let value = parser.parse_value()?;

    // A valid top-level bencode value must consume all input bytes.
    if !parser.is_at_end() {
        return Err(
            "trailing bytes after the top-level bencode value"
                .into(),
        );
    }

    bencode_to_json_value_bytes(&value)
}

/// Converts a bencode value into a JSON value.
fn bencode_to_json_value(
    value: &Bencode
) -> Result<Value, Box<dyn std::error::Error>> {
    match value {
        Bencode::Integer(value) => {
            Ok(Value::Number(Number::from(*value)))
        }

        Bencode::Bytes(bytes) => match std::str::from_utf8(bytes) {
            // Valid UTF-8 byte strings can be represented directly as JSON
            // strings.
            Ok(string) => Ok(Value::String(string.to_owned())),

            // Preserve non-UTF-8 bytes using the binary marker format.
            Err(_) => Ok(Value::String(format!(
                "[binary: {}]",
                hex::encode(bytes)
            ))),
        },

        Bencode::List(values) => {
            let mut result = Vec::with_capacity(values.len());

            for value in values {
                result.push(bencode_to_json_value(value)?);
            }

            Ok(Value::Array(result))
        }

        Bencode::Dict(entries) => {
            let mut object = Map::with_capacity(entries.len());

            for (key, value) in entries {
                let key = String::from_utf8(key.clone()).map_err(
                    |_| "bencode dictionary key is not valid UTF-8",
                )?;

                object.insert(key, bencode_to_json_value(value)?);
            }

            Ok(Value::Object(object))
        }
    }
}

/// A parser for bencode input.
struct Parser<'a> {
    /// The complete input being parsed.
    data: &'a [u8],

    /// The index of the next byte to parse.
    position: usize,
}

impl<'a> Parser<'a> {
    /// Creates a parser positioned at the beginning of `data`.
    fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    /// Returns `true` when every input byte has been consumed.
    fn is_at_end(&self) -> bool {
        self.position == self.data.len()
    }

    /// Parses one bencode value at the current parser position.
    fn parse_value(
        &mut self
    ) -> Result<Bencode, Box<dyn std::error::Error>> {
        let byte = *self
            .data
            .get(self.position)
            .ok_or("unexpected end of bencode data")?;

        match byte {
            b'i' => self.parse_integer(),
            b'l' => self.parse_list(),
            b'd' => self.parse_dict(),
            b'0'..=b'9' => self.parse_bytes(),

            _ => Err(format!(
                "invalid bencode byte 0x{byte:02x} at offset {}",
                self.position
            )
                .into()),
        }
    }

    /// Parses an integer in the form `i<number>e`.
    fn parse_integer(
        &mut self
    ) -> Result<Bencode, Box<dyn std::error::Error>> {
        // Skip the initial `i`.
        self.position += 1;

        let start = self.position;

        // Search for the terminating `e`.
        while let Some(byte) = self.data.get(self.position) {
            if *byte == b'e' {
                break;
            }

            self.position += 1;
        }

        if self.position >= self.data.len() {
            return Err("unterminated bencode integer".into());
        }

        let text =
            std::str::from_utf8(&self.data[start..self.position])?;
        let value = text.parse::<i64>()?;

        // Skip the terminating `e`.
        self.position += 1;

        Ok(Bencode::Integer(value))
    }

    /// Parses a byte string in the form `<length>:<bytes>`.
    fn parse_bytes(
        &mut self
    ) -> Result<Bencode, Box<dyn std::error::Error>> {
        let length_start = self.position;

        // Read the decimal length until the colon separator.
        while let Some(byte) = self.data.get(self.position) {
            if *byte == b':' {
                break;
            }

            if !byte.is_ascii_digit() {
                return Err("invalid byte-string length".into());
            }

            self.position += 1;
        }

        if self.position >= self.data.len() {
            return Err("unterminated byte-string length".into());
        }

        let length_text = std::str::from_utf8(
            &self.data[length_start..self.position],
        )?;

        let length = length_text.parse::<usize>()?;

        // Skip the colon.
        self.position += 1;

        let end = self
            .position
            .checked_add(length)
            .ok_or("byte-string length overflow")?;

        if end > self.data.len() {
            return Err(
                "byte-string extends past end of input".into()
            );
        }

        let bytes = self.data[self.position..end].to_vec();
        self.position = end;

        Ok(Bencode::Bytes(bytes))
    }

    /// Parses a list in the form `l<value>...e`.
    fn parse_list(
        &mut self
    ) -> Result<Bencode, Box<dyn std::error::Error>> {
        // Skip the initial `l`.
        self.position += 1;

        let mut values = Vec::new();

        loop {
            if self.peek_is(b'e') {
                // Skip the terminating `e`.
                self.position += 1;
                break;
            }

            values.push(self.parse_value()?);
        }

        Ok(Bencode::List(values))
    }

    /// Parses a dictionary in the form `d<key><value>...e`.
    fn parse_dict(
        &mut self
    ) -> Result<Bencode, Box<dyn std::error::Error>> {
        // Skip the initial `d`.
        self.position += 1;

        let mut entries = Vec::new();

        loop {
            if self.peek_is(b'e') {
                // Skip the terminating `e`.
                self.position += 1;
                break;
            }

            let key = match self.parse_value()? {
                Bencode::Bytes(key) => key,

                _ => {
                    return Err(
                        "bencode dictionary key is not a byte string".into()
                    );
                }
            };

            let value = self.parse_value()?;
            entries.push((key, value));
        }

        Ok(Bencode::Dict(entries))
    }

    /// Checks whether the next byte is `expected` without consuming it.
    fn peek_is(
        &self,
        expected: u8,
    ) -> bool {
        self.data
            .get(self.position)
            .copied()
            .is_some_and(|byte| byte == expected)
    }
}
