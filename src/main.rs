use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::process::exit;

const MAGIC: &[u8; 4] = b"TCMP";
const VERSION: u8 = 1;

// ---------------------------------------------------------------- tokenizer

fn kind(c: char) -> u8 {
    if c.is_alphanumeric() {
        0 // word
    } else if c.is_whitespace() {
        1 // whitespace
    } else {
        2 // punctuation
    }
}

fn tokenize(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();

    while let Some((start, c)) = chars.next() {
        let k = kind(c);
        let mut end = start + c.len_utf8();

        if k != 2 {
            while let Some(&(j, next)) = chars.peek() {
                if kind(next) == k {
                    end = j + next.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
        }
        tokens.push(&text[start..end]);
    }
    tokens
}

// ------------------------------------------------------------------ varints

fn write_varint(out: &mut Vec<u8>, mut v: u32) {
    while v >= 128 {
        out.push((v & 0x7F) as u8 | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn read_varint(data: &[u8], pos: &mut usize) -> Result<u32, String> {
    let mut result = 0u32;
    let mut shift = 0u32;
    loop {
        let b = *data.get(*pos).ok_or("unexpected end of data")?;
        *pos += 1;
        if shift >= 35 {
            return Err("varint too long".into());
        }
        result |= ((b & 0x7F) as u32) << shift;
        if b & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
    }
}

// ------------------------------------------------------------ encode/decode

fn encode(text: &str) -> Vec<u8> {
    let tokens = tokenize(text);

    let mut counts: HashMap<&str, u32> = HashMap::new();
    let mut dict: Vec<&str> = Vec::new();
    for &t in &tokens {
        let c = counts.entry(t).or_insert(0);
        if *c == 0 {
            dict.push(t);
        }
        *c += 1;
    }

    // most frequent first => smallest ids (stable sort keeps this deterministic)
    dict.sort_by_key(|t| Reverse(counts[t]));

    let ids: HashMap<&str, u32> = dict
        .iter()
        .enumerate()
        .map(|(i, &t)| (t, i as u32))
        .collect();

    // layout: MAGIC, VERSION, [dict count], per token [len][bytes],
    //         [id count], ids... (all numbers are varints)
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.push(VERSION);
    write_varint(&mut bytes, dict.len() as u32);
    for token in &dict {
        write_varint(&mut bytes, token.len() as u32);
        bytes.extend_from_slice(token.as_bytes());
    }
    write_varint(&mut bytes, tokens.len() as u32);
    for t in &tokens {
        write_varint(&mut bytes, ids[t]);
    }
    bytes
}

fn decode(data: &[u8]) -> Result<String, String> {
    if data.len() < 5 || &data[..4] != MAGIC {
        return Err("not a txtCompress file (bad magic bytes)".into());
    }
    if data[4] != VERSION {
        return Err(format!("unsupported format version {}", data[4]));
    }
    let mut pos = 5;

    let dict_len = read_varint(data, &mut pos)?;
    let mut dict: Vec<&str> = Vec::new();
    for _ in 0..dict_len {
        let len = read_varint(data, &mut pos)? as usize;
        let end = pos.checked_add(len).ok_or("corrupt length")?;
        let slice = data.get(pos..end).ok_or("unexpected end of data")?;
        dict.push(std::str::from_utf8(slice).map_err(|_| "invalid utf-8 in dictionary")?);
        pos = end;
    }

    let count = read_varint(data, &mut pos)?;
    let mut text = String::new();
    for _ in 0..count {
        let id = read_varint(data, &mut pos)? as usize;
        text.push_str(dict.get(id).ok_or("token id out of range (corrupt file)")?);
    }
    Ok(text)
}

// ---------------------------------------------------------------------- CLI

fn usage() -> ! {
    eprintln!(
        "txtCompress - tiny dictionary-based text compressor

USAGE:
    txtCompress -c FILE [-o OUTPUT] [-v]    compress
    txtCompress -u FILE [-o OUTPUT] [-v]    uncompress

    -c      compress
    -u      uncompress
    -o FILE choose the output name ('-' = stdout)
    -v      print size statistics
    -h      show this help

    If you leave out -o, the name is picked for you:
      -c notes.txt   ->  notes.txt.tcmp
      -u notes.txt.tcmp  ->  notes.txt
    With no FILE, it reads stdin and writes stdout (for pipes).

EXAMPLES:
    txtCompress -c input.txt
    txtCompress -u input.txt.tcmp
    txtCompress -c input.txt -v"
    );
    exit(2);
}

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("txtCompress: error: {msg}");
    exit(1);
}

fn main() {
    let mut args = std::env::args().skip(1);

    let mut mode: Option<bool> = None; // Some(true) = compress, Some(false) = uncompress
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut verbose = false;

    while let Some(a) = args.next() {
        match a.as_str() {
            "-c" | "--compress" => mode = Some(true),
            "-u" | "--uncompress" | "-d" => mode = Some(false),
            "-o" | "--output" => output = Some(args.next().unwrap_or_else(|| usage())),
            "-v" | "--verbose" => verbose = true,
            "-h" | "--help" => usage(),
            "-" => input = None,
            s if s.starts_with('-') => {
                eprintln!("txtCompress: unknown option '{s}'\n");
                usage();
            }
            _ => {
                if input.is_some() {
                    eprintln!("txtCompress: only one input file allowed\n");
                    usage();
                }
                input = Some(a);
            }
        }
    }
    let mode = mode.unwrap_or_else(|| usage());

    // no -o given but we have an input file: pick an output name automatically
    if output.is_none() {
        if let Some(inp) = &input {
            let auto = if mode {
                format!("{inp}.tcmp")
            } else if let Some(stripped) = inp.strip_suffix(".tcmp") {
                stripped.to_string()
            } else {
                format!("{inp}.out")
            };
            if std::path::Path::new(&auto).exists() {
                fail(format!(
                    "{auto} already exists (use -o to choose another name)"
                ));
            }
            eprintln!("writing {auto}");
            output = Some(auto);
        }
    } else if output.as_deref() == Some("-") {
        output = None; // explicit stdout
    }

    // read input
    let data = match &input {
        Some(path) => fs::read(path).unwrap_or_else(|e| fail(format!("{path}: {e}"))),
        None => {
            let mut buf = Vec::new();
            io::stdin()
                .read_to_end(&mut buf)
                .unwrap_or_else(|e| fail(format!("stdin: {e}")));
            buf
        }
    };

    // do the work
    let result: Vec<u8> = if mode {
        let text = String::from_utf8(data.clone())
            .unwrap_or_else(|_| fail("input is not valid UTF-8 text"));
        encode(&text)
    } else {
        decode(&data).unwrap_or_else(|e| fail(e)).into_bytes()
    };

    // write output
    match &output {
        Some(path) => fs::write(path, &result).unwrap_or_else(|e| fail(format!("{path}: {e}"))),
        None => {
            if mode && io::stdout().is_terminal() {
                fail("refusing to write binary data to a terminal (use -o FILE or a pipe)");
            }
            let mut out = io::stdout().lock();
            if let Err(e) = out.write_all(&result).and_then(|_| out.flush()) {
                // a closed pipe (e.g. `| head`) is not worth an error
                if e.kind() != io::ErrorKind::BrokenPipe {
                    fail(e);
                }
            }
        }
    }

    if verbose {
        let (orig, comp) = if mode {
            (data.len(), result.len())
        } else {
            (result.len(), data.len())
        };
        let ratio = if orig == 0 {
            0.0
        } else {
            comp as f64 / orig as f64 * 100.0
        };
        eprintln!("original:   {orig} bytes");
        eprintln!("compressed: {comp} bytes");
        eprintln!("ratio:      {ratio:.1}%");
    }
}
