use {
    anyhow::{anyhow, Result},
    charming::{
        component::{RadarCoordinate, Title},
        series::Radar,
        Chart, ImageRenderer,
    },
    clap::Parser,
    lazy_static::lazy_static,
    serde::Deserialize,
    std::collections::HashMap,
    std::{fmt::Debug, fs::read_dir, path::Path, process::Command},
};

fn visit_dirs(
    dir: &Path,
    username: &str,
    accumulator: Option<HashMap<String, usize>>,
) -> Result<Option<HashMap<String, usize>>> {
    if dir.is_dir() {
        let mut inner_accumulator = None;
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
                    let res =
                        parse_repo(path.parent().ok_or(anyhow!("no parent folder"))?, username)?;
                    inner_accumulator = combine_loc_by_lang(Some(res), inner_accumulator)
                } else {
                    inner_accumulator = visit_dirs(&path, username, inner_accumulator.clone())?;
                }
            };
        }
        Ok(combine_loc_by_lang(accumulator, inner_accumulator))
    } else {
        Err(anyhow!("input is not a directory"))
    }
}

fn combine_loc_by_lang(
    accumulator: Option<HashMap<String, usize>>,
    other_accumulator: Option<HashMap<String, usize>>,
) -> Option<HashMap<String, usize>> {
    match (accumulator, other_accumulator) {
        (Some(mut loc_by_lang), Some(loc_by_lang_inner)) => {
            for (lang, loc) in loc_by_lang_inner.iter() {
                loc_by_lang
                    .entry(lang.clone())
                    .and_modify(|count| *count += loc)
                    .or_insert(*loc);
            }
            Some(loc_by_lang)
        }
        (Some(loc_by_lang), None) => Some(loc_by_lang),
        (None, Some(loc_by_lang_inner)) => Some(loc_by_lang_inner),
        (None, None) => None,
    }
}

fn parse_repo(dir: &Path, username: &str) -> Result<HashMap<String, usize>> {
    println!("Parsing {:?}", dir.as_os_str());
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
    // println!("{:?}", splitted);

    let loc_by_lang = splitted.iter().try_fold(
        HashMap::new(),
        |mut acc, path| -> Result<HashMap<String, usize>> {
            let blame_output = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args([
                        "/C",
                        &format!(
                            "cd {} && git blame --line-porcelain {path}",
                            dir.to_str().unwrap()
                        ),
                    ])
                    .output()
                    .expect("failed to execute process")
            } else {
                Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        "cd {} && git blame --line-porcelain {path}",
                        dir.to_str().unwrap()
                    ))
                    .output()
                    .expect("failed to execute process")
            };
            let Ok(blame) = String::from_utf8(blame_output.stdout) else {
                return Ok(acc);
            };
            let count = blame.matches(&format!("author {username}")).count();

            if let Some(extension) = Path::new(path).extension() {
                let extension_with_dot = String::from(".")
                    + extension
                        .to_str()
                        .ok_or(anyhow!("found None in Os string"))?;

                if let Some(lang) = LANGUAGES.get(&extension_with_dot) {
                    acc.entry(lang.name.clone())
                        .and_modify(|lang_count| *lang_count += count)
                        .or_insert(count);
                }
                Ok(acc)
            } else {
                Ok(acc)
            }
        },
    )?;
    println!("We have {:?} loc in {}", loc_by_lang, dir.display());

    Ok(loc_by_lang)
}

fn chart(data: &HashMap<String, usize>, username: &str) -> Result<Chart> {
    let max_loc = data
        .values()
        .max()
        .ok_or(anyhow!("no data to render"))?
        .to_owned() as i64;
    let radar_triplets: Vec<(&str, i64, i64)> = data
        .iter()
        .map(|(lang, _)| (lang.as_str(), 0, max_loc))
        .collect();
    let locs: Vec<i64> = data.iter().map(|(_, loc)| loc.to_owned() as i64).collect();
    Ok(Chart::new()
        .title(Title::new().text(format!("{username}'s tech stack")))
        .radar(RadarCoordinate::new().indicator(radar_triplets))
        .series(Radar::new().name("LOC").data(vec![(locs, "Foo")])))
}

#[derive(Deserialize, Debug, Clone)]
struct Language {
    name: String,
    r#type: String,
    extensions: Vec<String>,
}

lazy_static! {
    // File extension is the key
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

    // TODO: add functionality
    /// Depth  of child directories to traverse
    #[arg(short, long)]
    depth: Option<u8>,

    // Git username. If none provided, the global git username will be used
    #[arg(short, long)]
    author: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let path = if let Some(path) = &args.path {
        Path::new(path).to_owned()
    } else {
        std::env::current_dir()?
    };
    let username = args.author.unwrap_or_else(|| {
        let username_output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "git config --global user.name"])
                .output()
                .expect("failed to execute process")
        } else {
            Command::new("sh")
                .arg("-c")
                .arg("git config --global user.name")
                .output()
                .expect("failed to execute process")
        };
        let username = String::from_utf8(username_output.stdout).unwrap();
        username.trim().to_owned()
    });

    let res = visit_dirs(path.as_path(), &username, None)?;
    println!("{:?}", res);

    if let Some(data) = res {
        let radar = chart(&data, &username)?;
        let mut renderer = ImageRenderer::new(1000, 800);
        renderer.save(&radar, "radar.svg");
    }
    Ok(())
}
