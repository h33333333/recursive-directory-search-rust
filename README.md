# Recursive directory search written in Rust
## Usage
Scanning the `test-directory` folder for files that have `key`:
```sh
$ cargo run -- test-directory/ key
test-directory/file
test-directory/sub-directory/file.txt
test-directory/sub-directory/sub-directory/file
```
`test-directory` is included in this repo to make testing easier.
## Test data structure
```sh
$ exa --tree test-directory/
test-directory
├── file
└── sub-directory
   ├── file.txt
   ├── file_2.txt
   └── sub-directory
      └── file
```
All files, but file_2.txt, contain single line - `key` and are plaintext files.
