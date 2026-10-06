# txtCompress

A small hobby text compressor written in Rust. It splits text into tokens (words, whitespace runs, punctuation), builds a dictionary of the unique tokens sorted by how often they appear, and stores the text as a list of small dictionary ids. Everything is encoded as variable-length integers, so common tokens take only one byte.

It is a learning project, not a replacement for gzip or zstd.

## Project layout

```
txtcomp/
├── Cargo.toml
├── README.md
├── install.bat      (Windows installer)
├── install.sh       (Linux / macOS / WSL installer)
└── src/
    └── main.rs
```

The install scripts must stay next to `Cargo.toml`, not inside `src`.

## Install

You need [Rust](https://rustup.rs) first.

### Windows

1. Install Rust from https://rustup.rs (run `rustup-init.exe`).
2. Double-click `install.bat`.
3. Open a **new** terminal and run `txtCompress -h`.

### Linux / macOS / WSL

1. Install Rust:
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   On Debian/Ubuntu you may also need `sudo apt install build-essential`.
2. From the project folder run:
   ```
   sh install.sh
   ```
3. Open a new terminal and run `txtCompress -h`.

### Manual install

```
cargo install --path .
```

Or build without installing (the program ends up in `target/release/`):

```
cargo build --release
```

### Troubleshooting

- **"command not found" / "not recognized":** open a new terminal. On Linux you can also add `~/.cargo/bin` to your PATH:
  ```
  echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
  ```
- **"couldn't read main.rs":** you ran `rustc` in the wrong folder. Use `cargo install --path .` from the folder that contains `Cargo.toml`.
- **Linker errors on Windows:** install the Visual Studio C++ Build Tools, or use WSL.

## Usage

```
txtCompress -c FILE [-o OUTPUT] [-v]    compress
txtCompress -u FILE [-o OUTPUT] [-v]    uncompress
```

| Option | Meaning |
|--------|---------|
| `-c`   | compress |
| `-u`   | uncompress (`-d` also works) |
| `-o`   | choose the output file name (`-` means stdout) |
| `-v`   | print original size, compressed size and ratio |
| `-h`   | show help |

### Examples

```
txtCompress -c input.txt                       # creates input.txt.tcmp
txtCompress -u input.txt.tcmp -o restored.txt  # gets the text back
txtCompress -c input.txt -v                    # also shows the savings
```

### Automatic output names

If you leave out `-o`:

- `-c notes.txt` writes `notes.txt.tcmp`
- `-u notes.txt.tcmp` writes `notes.txt`
- `-u` on a file that does not end in `.tcmp` writes `FILE.out`

It never overwrites an existing file. If the output name is taken, pick another with `-o`. Because uncompressing `notes.txt.tcmp` targets `notes.txt`, use `-o` if the original is still in the folder.

### Pipes (Linux, macOS, WSL, cmd.exe)

With no input file it reads stdin and writes stdout:

```
cat notes.txt | txtCompress -c | txtCompress -u
```

In PowerShell, `|` and `>` corrupt binary data. Use `-o` there. Compressing never prints binary to a terminal; use `-o` or a pipe.

## Check that it works

```
txtCompress -c input.txt -v
txtCompress -u input.txt.tcmp -o restored.txt
fc input.txt restored.txt        # Windows
cmp input.txt restored.txt       # Linux
```

No differences means the round trip is lossless.

## File format

All numbers are varints (7 bits per byte, high bit means "more bytes follow").

```
"TCMP"            4 bytes, magic
version           1 byte (currently 1)
dict_count        varint
  per entry:      length (varint) + UTF-8 bytes
token_count       varint
  per token:      dictionary id (varint)
```

## Limitations

- Text only: input must be valid UTF-8.
- Small files can get bigger, because the dictionary costs more than it saves.
- It only exploits repeated tokens. It does not find repeated phrases or use entropy coding like Huffman, so ratios are modest.
- Damaged files, or files that did not come from this tool, are rejected with an error instead of crashing.

## Ideas for later

- Huffman or arithmetic coding for the ids
- Store the raw text when compression would make it bigger
- Dictionary entries for common word pairs
- Stream large files instead of loading them fully
- Prebuilt `.exe` downloads so Rust isn't needed