use {
    anyhow::{anyhow, Result},
    clap::Parser,
    lazy_static::lazy_static,
    serde::Deserialize,
    std::collections::HashMap,
    std::{fmt::Debug, fs::read_dir, path::Path, process::Command},
};

fn visit_dirs(dir: &Path) -> Result<()> {
    if dir.is_dir() {
        for entry in read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path
                    .file_name()
                    .ok_or(anyhow!("directory without a name"))?
                    .to_str()
                    .ok_or(anyhow!("directory name is not valid unicode"))?
                    == ".git"
                {
                    parse_repo(path.parent().ok_or(anyhow!("no parent folder"))?)?
                }
                visit_dirs(&path)?;
            }
        }
        Ok(())
    } else {
        Err(anyhow!("input is not a directory"))
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Language {
    name: String,
    r#type: String,
    extensions: Vec<String>,
}

fn parse_repo(dir: &Path) -> Result<()> {
    println!("{:?}", dir.as_os_str());
    let ls_files_output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args([
                "/C",
                &format!("cd {} && git ls-files", dir.to_str().unwrap()),
            ])
            .output()
            .expect("cannot invoke git ls-files")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("cd {} && git ls-files", dir.to_str().unwrap()))
            .output()
            .expect("cannot invoke git ls-files")
    };

    let git_files = String::from_utf8(ls_files_output.stdout)?;
    let mut splitted: Vec<&str> = git_files.split("\n").collect();
    // the last element is always an empty line
    splitted.pop();
    println!("{:?}", splitted);

    // splitted.iter().map(|path| {

    //     let output = if cfg!(target_os = "windows") {
    //     Command::new("cmd")
    //         .args(["/C", "echo hello"])
    //         .output()
    //         .expect("failed to execute process")
    //     } else {
    //     Command::new("sh")
    //         .arg("-c")
    //         .arg("echo hello")
    //         .output()
    //         .expect("failed to execute process")
    //     };
    //         })

    Ok(())
}

lazy_static! {
    static ref LANGUAGES: HashMap<String, Language> = {
        let languages_asset = include_str!(
            "../43962d06686722d26d176fad46879d41/Programming_Languages_Extensions.json"
        );
        let languages_vec: Vec<Language> = serde_json::from_str(languages_asset).unwrap();
        let mut languages_map = HashMap::new();
        for lang in languages_vec {
            for ext in lang.extensions.clone() {
                languages_map.insert(ext, lang.clone());
            }
        }
        languages_map
    };
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Which path to search
    #[arg(short, long)]
    path: Option<String>,

    /// Depth  of child directories to traverse
    #[arg(short, long)]
    depth: Option<u8>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let path = if let Some(path) = &args.path {
        Path::new(path).to_owned()
    } else {
        std::env::current_dir()?
    };

    visit_dirs(path.as_path())?;
    Ok(())
}
