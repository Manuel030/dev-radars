# dev-radars
Render a radar plot of your tech stack. `dev-radars` parses git objects to compute statistics per technology.

`dev-radars` is something between running [Aloc](https://github.com/AlDanial/cloc/tree/master) in a Docker container and executing random commands from the internet:

```
git ls-files | xargs -n1 git blame --line-porcelain | sed -n 's/^author //p' | sort -f | uniq -ic | sort -nr
```

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

