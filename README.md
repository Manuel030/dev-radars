# dev-radars
Render a radar plot of your tech stack. `dev-radars` parses git objects to compute statistics per technology.

## Installation

Using cargo:

```
git clone git@github.com:Manuel030/dev-radars.git
cargo install --path dev-radars
```

## Usage
```
Usage: dev-radars [OPTIONS]

Options:
  -p, --path <PATH>      Which path to search
  -d, --depth <DEPTH>    Depth  of child directories to traverse
  -a, --author <AUTHOR>  
  -h, --help             Print help
  -V, --version          Print version
```

## Todos
- [ ] Flag to add an ignore list of directory names
- [ ] Parallel processing
- [ ] Installation with Nix

