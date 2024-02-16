use {
    anyhow::{anyhow, Result},
    charming::{
        component::RadarCoordinate, element::AreaStyle, series::Radar, Chart, ImageRenderer,
    },
    clap::Parser,
    lazy_static::lazy_static,
    serde::Deserialize,
    std::{collections::HashMap, fmt::Debug, fs::read_dir, path::Path, process::Command},
};

fn visit_dirs(
    dir: &Path,
    usernames: &Vec<&str>,
    accumulator: Option<HashMap<String, i64>>,
    max_depth: Option<u8>,
    mut current_depth: Option<u8>,
) -> Result<Option<HashMap<String, i64>>> {
    if dir.is_dir() {
        current_depth =
            if let (Some(max_depth), Some(mut current_depth)) = (max_depth, current_depth) {
                if current_depth > max_depth {
                    return Ok(None);
                } else {
                    current_depth += 1
                }
                Some(current_depth)
            } else {
                current_depth
            };
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
                        parse_repo(path.parent().ok_or(anyhow!("no parent folder"))?, usernames)?;
                    inner_accumulator = combine_loc_by_lang(Some(res), inner_accumulator)
                } else {
                    let res = visit_dirs(&path, usernames, None, max_depth, current_depth)?;
                    inner_accumulator = combine_loc_by_lang(inner_accumulator, res);
                }
            };
        }
        Ok(combine_loc_by_lang(accumulator, inner_accumulator))
    } else {
        Err(anyhow!("input is not a directory"))
    }
}

fn combine_loc_by_lang(
    accumulator: Option<HashMap<String, i64>>,
    other_accumulator: Option<HashMap<String, i64>>,
) -> Option<HashMap<String, i64>> {
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

fn parse_repo(dir: &Path, usernames: &Vec<&str>) -> Result<HashMap<String, i64>> {
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
        |mut acc: HashMap<String, i64>, path| -> Result<HashMap<String, i64>> {
            let blame_output = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args([
                        "/C",
                        &format!(
                            "cd {} && git blame --line-porcelain {}",
                            dir.to_str().unwrap(),
                            path
                        ),
                    ])
                    .output()
                    .expect("failed to execute process")
            } else {
                Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        "cd {} && git blame --line-porcelain {}",
                        dir.to_str().unwrap(),
                        path
                    ))
                    .output()
                    .expect("failed to execute process")
            };
            let Ok(blame) = String::from_utf8(blame_output.stdout) else {
                return Ok(acc);
            };

            let count: i64 = usernames
                .iter()
                .map(|name| {
                    let c = blame.matches(&format!("author {name}")).count();
                    c as i64
                })
                .sum();

            if let Some(extension) = Path::new(path).extension() {
                let extension_with_dot = String::from(".")
                    + extension
                        .to_str()
                        .ok_or(anyhow!("found None in Os string"))?;

                if let Some(lang) = LANGUAGES.get(&extension_with_dot) {
                    if matches!(lang.r#type, LanguageType::Programming) && count > 0 {
                        acc.entry(lang.name.clone())
                            .and_modify(|lang_count| *lang_count += count)
                            .or_insert(count);
                    }
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

fn chart(data: &HashMap<String, i64>, top_n: u8) -> Result<Chart> {
    let max_loc = data
        .values()
        .max()
        .ok_or(anyhow!("no data to render"))?
        .to_owned();

    let mut sorted_by_loc: Vec<(&str, i64)> = data
        .iter()
        .map(|(lang, loc)| (lang.as_str(), *loc))
        .collect();
    sorted_by_loc.sort_unstable_by_key(|(_, loc)| *loc);
    sorted_by_loc.reverse();
    sorted_by_loc.truncate(top_n as usize);

    let radar_triplets: Vec<(&str, i64, i64)> = sorted_by_loc
        .iter()
        .map(|(lang, _)| (*lang, 0, max_loc))
        .collect();

    let locs: Vec<i64> = sorted_by_loc.iter().map(|(_, loc)| *loc).collect();

    Ok(Chart::new()
        .radar(RadarCoordinate::new().indicator(radar_triplets))
        .series(
            Radar::new()
                .name("LOC")
                .data(vec![(locs, "LOC")])
                .area_style(AreaStyle::new()),
        ))
}

#[derive(Deserialize, Debug, Clone)]
struct Language {
    name: String,
    r#type: LanguageType,
    extensions: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum LanguageType {
    Programming,
    Data,
    Prose,
    Markup,
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

    /// Depth of child directories to traverse
    #[arg(short, long)]
    depth: Option<u8>,

    // Git username(s). If none provided, the global git username will be used
    #[arg(short, long, value_delimiter = ' ',  num_args = 1..)]
    author: Option<Vec<String>>,

    // Top N languages to plot
    #[arg(short, long, default_value_t = 10)]
    top_n: u8,
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
        vec![username.trim().to_owned()]
    });

    let username_str = username.iter().map(|name| name.as_str()).collect();

    let res = visit_dirs(path.as_path(), &username_str, None, args.depth, Some(1))?;
    println!("{:?}", res);

    if let Some(data) = res {
        let radar = chart(&data, args.top_n)?;
        let mut renderer = ImageRenderer::new(1000, 800);
        let _ = renderer.save(&radar, "radar.svg");
    }
    Ok(())
}
