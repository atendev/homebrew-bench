use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn skill_dir() -> PathBuf {
    repo().join(".agents/skills/homebrew-bench")
}

fn skill_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|x| x == "md") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&skill_dir(), &mut out);
    out.sort();
    out
}

struct Flag {
    name: String,
    required: bool,
    values: Vec<String>,
}

/// Flags per command path, such as `"start"` or `"fixture show"`, from `bench help --json`.
fn catalog() -> BTreeMap<String, Vec<Flag>> {
    let out = Command::new(env!("CARGO_BIN_EXE_bench")).args(["help", "--json"]).output().unwrap();
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut map = BTreeMap::new();
    fn add(prefix: &str, cmd: &Value, map: &mut BTreeMap<String, Vec<Flag>>) {
        let name = format!("{prefix}{}", cmd["name"].as_str().unwrap());
        let flags = cmd["args"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| !a["positional"].as_bool().unwrap())
            .map(|a| Flag {
                name: a["name"].as_str().unwrap().to_string(),
                required: a["required"].as_bool().unwrap(),
                values: a["values"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect(),
            })
            .collect();
        map.insert(name.clone(), flags);
        for sub in cmd["subcommands"].as_array().into_iter().flatten() {
            add(&format!("{name} "), sub, map);
        }
    }
    for cmd in json["data"]["commands"].as_array().unwrap() {
        add("", cmd, &mut map);
    }
    map
}

/// Every `bench ...` invocation in inline code spans and fenced code blocks.
fn invocations(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            let line = line.trim();
            if line.starts_with("bench ") {
                out.push(line.to_string());
            }
            continue;
        }
        for (i, span) in line.split('`').enumerate() {
            if i % 2 == 1 && span.starts_with("bench ") {
                out.push(span.to_string());
            }
        }
    }
    out
}

#[test]
fn skill_commands_and_flags_exist_in_the_cli() {
    let catalog = catalog();
    let mut problems = Vec::new();
    let mut seen = 0;
    for file in skill_files() {
        let text = std::fs::read_to_string(&file).unwrap();
        for call in invocations(&text) {
            seen += 1;
            let words: Vec<&str> = call.split_whitespace().skip(1).collect();
            let Some(first) = words.first() else { continue };
            let two = words.get(1).map(|w| format!("{first} {w}"));
            let command = match two {
                Some(two) if catalog.contains_key(&two) => two,
                _ if catalog.contains_key(*first) => first.to_string(),
                _ => {
                    problems.push(format!("{}: unknown command in `{call}`", file.display()));
                    continue;
                }
            };
            let flags = &catalog[&command];
            for (i, word) in words.iter().enumerate() {
                let Some(name) = word.split('=').next().filter(|w| w.starts_with("--")) else { continue };
                let name = name.trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
                if name == "--help" {
                    continue;
                }
                let Some(flag) = flags.iter().find(|f| f.name == name) else {
                    problems.push(format!("{}: `{command}` has no `{name}` in `{call}`", file.display()));
                    continue;
                };
                let value = word.split_once('=').map(|(_, v)| v).or_else(|| words.get(i + 1).copied());
                if let Some(value) = value.filter(|v| !flag.values.is_empty() && !v.starts_with('<')) {
                    if !flag.values.iter().any(|v| v == value) {
                        problems.push(format!("{}: `{name}` takes {:?}, not `{value}`, in `{call}`", file.display(), flag.values));
                    }
                }
            }
            let words_in_command = command.split(' ').count();
            if words.len() > words_in_command {
                for flag in flags.iter().filter(|f| f.required) {
                    if !words.iter().any(|w| w.split('=').next() == Some(flag.name.as_str())) {
                        problems.push(format!("{}: `{call}` lacks required `{}`", file.display(), flag.name));
                    }
                }
            }
        }
    }
    assert!(seen > 20, "expected the skill to show its commands, found {seen}");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn skill_frontmatter_holds_only_name_and_description() {
    let text = std::fs::read_to_string(skill_dir().join("SKILL.md")).unwrap();
    let front = text.strip_prefix("---\n").and_then(|t| t.split_once("\n---\n")).expect("frontmatter").0;
    let keys: Vec<&str> = front
        .lines()
        .filter(|l| !l.starts_with(' ') && !l.is_empty())
        .map(|l| l.split(':').next().unwrap())
        .collect();
    assert_eq!(keys, ["name", "description"]);
    assert!(front.lines().any(|l| l == "name: homebrew-bench"));
}

#[test]
fn skill_links_resolve() {
    for file in skill_files() {
        let text = std::fs::read_to_string(&file).unwrap();
        for part in text.split("](").skip(1) {
            let target = part.split(')').next().unwrap();
            if target.starts_with("http") || target.starts_with('#') {
                continue;
            }
            let path = file.parent().unwrap().join(target.split('#').next().unwrap());
            assert!(path.exists(), "{}: broken link {target}", file.display());
        }
    }
}

#[test]
fn claude_symlink_and_agents_md_point_at_the_skill() {
    let link = repo().join(".claude/skills/homebrew-bench");
    let target = std::fs::read_link(&link).expect(".claude/skills/homebrew-bench is a symlink");
    assert_eq!(target, Path::new("../../.agents/skills/homebrew-bench"));
    assert!(link.join("SKILL.md").exists());
    let agents = std::fs::read_to_string(repo().join("AGENTS.md")).unwrap();
    assert!(agents.contains(".agents/skills/homebrew-bench/SKILL.md"));
}

#[test]
fn house_rule_template_has_the_frontmatter_and_four_sections() {
    let text = std::fs::read_to_string(skill_dir().join("references/house-rule-template.md")).unwrap();
    for needle in ["status: active", "kind: gap", "engine: none", "## Rule", "## Applies when", "## How to apply", "## Notes"] {
        assert!(text.contains(needle), "template lacks {needle}");
    }
}
