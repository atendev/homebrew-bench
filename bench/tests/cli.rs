use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Bench {
    home: PathBuf,
    cwd: PathBuf,
}

impl Bench {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("bench-cli-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("home");
        let cwd = root.join("work");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&cwd).unwrap();
        Self { home, cwd }
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let out = Command::new(env!("CARGO_BIN_EXE_bench"))
            .args(args)
            .env("BENCH_HOME", &self.home)
            .current_dir(&self.cwd)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let json = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{args:?}: {e}\n{text}"));
        (out.status.code().unwrap_or(-1), json)
    }

    fn ok(&self, args: &[&str]) -> Value {
        let (code, json) = self.run(args);
        assert_eq!(code, 0, "{args:?}: {json:#}");
        json["data"].clone()
    }

    fn err(&self, args: &[&str]) -> String {
        let (code, json) = self.run(args);
        assert_ne!(code, 0, "{args:?} should fail: {json:#}");
        json["error"]["code"].as_str().unwrap().to_string()
    }

    fn start(&self, extra: &[&str]) -> Value {
        let pc = data("party/standard/standard-striker-03.json");
        let goblin = data("fixtures/goblin-warrior.json");
        let mut args = vec!["start", pc.as_str(), goblin.as_str(), "--seed", "11"];
        args.extend_from_slice(extra);
        self.ok(&args)
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.home.parent().unwrap());
    }
}

fn data(rel: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../data").join(rel).display().to_string()
}

fn attack(bench: &Bench) -> Vec<String> {
    let state = bench.ok(&["state"]);
    let actor = state["digest"]["active_turn"].as_str().unwrap().to_string();
    let (action, target) = if actor == "standard-striker-03" {
        ("longbow", "goblin-warrior")
    } else {
        ("shortbow", "standard-striker-03")
    };
    vec!["action".into(), actor, action.into(), target.into()]
}

fn strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

#[test]
fn start_writes_state_log_and_index() {
    let bench = Bench::new();
    let out = bench.start(&[]);
    assert_eq!(out["encounter"], "goblin-warrior-1");
    assert!(out["digest"].is_object());
    assert!(out["events"].as_array().unwrap().len() > 2);
    let state = bench.home.join("encounters/goblin-warrior-1.json");
    let log = bench.home.join("encounters/goblin-warrior-1.log");
    assert!(state.is_file() && log.is_file());
    let list = bench.ok(&["list"]);
    assert_eq!(list["encounters"][0]["id"], "goblin-warrior-1");

    let second = bench.start(&["--output", "workspace"]);
    assert_eq!(second["encounter"], "goblin-warrior-2");
    assert!(bench.cwd.join(".bench/encounters/goblin-warrior-2.json").is_file());
}

#[test]
fn undo_then_reissue_gets_the_same_dice_and_replay_matches() {
    let bench = Bench::new();
    bench.start(&[]);
    let cmd = attack(&bench);
    let first = bench.ok(&strs(&cmd));
    let undo = bench.ok(&["undo"]);
    assert!(undo["events"][0]["line"].as_str().unwrap().starts_with("UNDO reverts entry [1]"));
    let again = bench.ok(&strs(&cmd));
    assert_eq!(first["events"], again["events"]);
    bench.ok(&["end-turn"]);
    let replay = bench.ok(&["replay"]);
    assert_eq!(replay["matches"], true, "{replay:#}");
    assert_eq!(replay["commands"], 4);
}

#[test]
fn hand_edited_state_is_refused() {
    let bench = Bench::new();
    bench.start(&[]);
    let path = bench.home.join("encounters/goblin-warrior-1.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut state: Value = serde_json::from_str(&text).unwrap();
    state["combatants"][1]["hp"] = Value::from(1);
    state["combatants"][0]["hp"] = Value::from(1);
    std::fs::write(&path, serde_json::to_string(&state).unwrap()).unwrap();
    assert_eq!(bench.err(&["state"]), "state_tampered");
    assert_eq!(bench.err(&["end-turn"]), "state_tampered");
}

#[test]
fn override_validates_paths_and_public_view_hides_values() {
    let bench = Bench::new();
    bench.start(&[]);
    bench.ok(&["override", "goblin-warrior.hp", "2", "--reason", "earlier fight"]);
    bench.ok(&["override", "standard-striker-03.conditions", r#"["prone"]"#, "--reason", "tripped"]);
    bench.ok(&["override", "standard-striker-03.conditions", "--clear", "--reason", "stood"]);
    assert_eq!(bench.err(&["override", "goblin-warrior.id", "x", "--reason", "r"]), "invalid_override");
    assert_eq!(bench.err(&["override", "engine_version", "\"9\"", "--reason", "r"]), "invalid_override");
    assert_eq!(bench.err(&["override", "goblin-warrior.hp", "lots", "--reason", "r"]), "invalid_override");
    assert_eq!(bench.err(&["override", "nobody.hp", "3", "--reason", "r"]), "invalid_override");

    let public = bench.ok(&["state", "--public"]);
    let goblin = public["digest"]["combatants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "goblin-warrior")
        .unwrap()
        .clone();
    assert_eq!(goblin["wounded"], "at death's door");
    assert!(goblin.get("hp").is_none());

    let log = bench.ok(&["log", "--public"])["log"].as_str().unwrap().to_string();
    assert!(log.contains("OVERRIDE goblin-warrior.hp (reason: earlier fight)"), "{log}");
    assert!(!log.contains("= 2"), "{log}");
    let full = bench.ok(&["log"])["log"].as_str().unwrap().to_string();
    assert!(full.contains("OVERRIDE goblin-warrior.hp = 2 (reason: earlier fight)"), "{full}");
}

#[test]
fn house_rule_slugs_are_checked() {
    let bench = Bench::new();
    bench.start(&[]);
    assert_eq!(bench.err(&["roll", "1d6", "--reason", "r", "--rule", "flanking"]), "unknown_house_rule");
    let rules = bench.cwd.join(".bench/house-rules");
    std::fs::create_dir_all(&rules).unwrap();
    std::fs::write(rules.join("flanking.md"), "# Flanking\n").unwrap();
    let out = bench.ok(&["roll", "1d6", "--reason", "r", "--rule", "flanking"]);
    let line = out["events"][0]["line"].as_str().unwrap();
    assert!(line.starts_with("AGENT JUDGMENT roll 1d6 = "), "{line}");
    assert!(line.ends_with("[house rule: flanking]"), "{line}");
}

#[test]
fn pending_prompt_gates_commands() {
    let bench = Bench::new();
    let started = bench.start(&["--pc-dice", "ask"]);
    assert_eq!(started["pending"]["actor_id"], "standard-striker-03");
    assert_eq!(started["pending"]["sides"], 20);
    assert_eq!(bench.err(&["end-turn"]), "pending_prompt");
    bench.ok(&["respond", "12"]);
    if bench.ok(&["state"])["digest"]["active_turn"] != "standard-striker-03" {
        bench.ok(&["end-turn"]);
    }
    let asked = bench.ok(&strs(&attack(&bench)));
    let prompt = asked["pending"].clone();
    assert_eq!(prompt["kind"], "dice");
    assert_eq!(bench.err(&["end-turn"]), "pending_prompt");
    assert_eq!(bench.err(&["respond", "99"]), "invalid_argument");
    bench.ok(&["respond", "--decline"]);
    bench.ok(&["end-turn"]);
    assert_eq!(bench.ok(&["replay"])["matches"], true);
}

#[test]
fn standing_override_tags_the_log() {
    let bench = Bench::new();
    bench.start(&[]);
    let set = bench.ok(&["override", "goblin-warrior.hp", "5", "--reason", "wounded scout", "--standing"]);
    let line = set["events"][0]["line"].as_str().unwrap();
    assert!(line.ends_with("[standing assumption]"), "{line}");
    let fought = attack(&bench);
    let lines = bench.ok(&strs(&fought));
    let first = lines["events"][0]["line"].as_str().unwrap();
    assert!(first.ends_with("[standing assumption: goblin-warrior.hp]"), "{first}");
}

#[test]
fn roster_changes_end_and_clean() {
    let bench = Bench::new();
    bench.start(&[]);
    let goblin = data("fixtures/goblin-warrior.json");
    let added = bench.ok(&["add", &goblin, "--side", "monsters", "--engaged-with", "standard-striker-03"]);
    let line = added["events"][0]["line"].as_str().unwrap();
    assert!(line.starts_with("ADD goblin-warrior-2 (Goblin Warrior 2) joins monsters"), "{line}");
    let removed = bench.ok(&["remove", "goblin-warrior-2", "--reason", "fled"]);
    assert_eq!(removed["events"][0]["line"], "REMOVE goblin-warrior-2 (reason: fled)");

    bench.ok(&["end", "--reason", "goblin surrendered"]);
    assert_eq!(bench.err(&["end-turn", "-e", "goblin-warrior-1"]), "encounter_ended");
    assert_eq!(bench.err(&["state"]), "not_found");
    assert!(bench.ok(&["state", "-e", "goblin-warrior-1"])["digest"].is_object());

    let cleaned = bench.ok(&["clean"]);
    assert_eq!(cleaned["removed"][0], "goblin-warrior-1");
    assert!(!bench.home.join("encounters/goblin-warrior-1.json").exists());
    assert!(bench.home.join("encounters/goblin-warrior-1.log").exists());
    assert_eq!(bench.ok(&["list"])["encounters"].as_array().unwrap().len(), 0);
}

#[test]
fn keep_encounters_false_prunes_ended_fights_at_next_start() {
    let bench = Bench::new();
    std::fs::write(bench.home.join("config.toml"), "keep_encounters = false\n").unwrap();
    bench.start(&[]);
    bench.ok(&["end", "--reason", "done"]);
    let undo = bench.ok(&["undo", "-e", "goblin-warrior-1"]);
    assert!(undo["events"][0]["line"].as_str().unwrap().starts_with("UNDO"));
    bench.ok(&["end", "--reason", "done"]);
    bench.start(&[]);
    assert!(!bench.home.join("encounters/goblin-warrior-1.json").exists());
    assert!(bench.home.join("encounters/goblin-warrior-1.log").exists());
    let ids: Vec<Value> = bench.ok(&["list"])["encounters"].as_array().unwrap().iter().map(|e| e["id"].clone()).collect();
    assert_eq!(ids, vec![Value::from("goblin-warrior-2")]);
}

#[test]
fn public_log_hides_seed_and_damage_amounts() {
    let bench = Bench::new();
    bench.start(&[]);
    for _ in 0..6 {
        let cmd = attack(&bench);
        bench.ok(&strs(&cmd));
        bench.ok(&["end-turn"]);
    }
    let public = bench.ok(&["log", "--public"]);
    let text = public["log"].as_str().unwrap();
    assert!(public.get("log_path").is_none());
    assert!(!text.contains("seed"), "{text}");
    for line in text.lines().filter(|l| l.contains("goblin-warrior takes")) {
        assert!(line.contains("goblin-warrior takes damage ("), "{line}");
    }
    assert!(bench.ok(&["log"])["log"].as_str().unwrap().contains("seed 11"));
}

#[test]
fn help_json_lists_every_command() {
    let bench = Bench::new();
    let catalog = bench.ok(&["help", "--json"]);
    let names: Vec<&str> = catalog["commands"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    for name in [
        "start", "action", "move", "respond", "end-turn", "end", "state", "list", "clean", "override", "roll", "add",
        "remove", "undo", "replay", "log",
    ] {
        assert!(names.contains(&name), "{name} missing from {names:?}");
    }
}
