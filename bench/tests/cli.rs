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
fn nimble_escape_disengages_as_a_bonus_action_with_no_target() {
    let bench = Bench::new();
    let start = bench.start(&[]);
    assert_eq!(start["digest"]["active_turn"], "goblin-warrior");
    let out = bench.ok(&["action", "goblin-warrior", "nimble-escape"]);
    assert_eq!(out["events"][0]["line"], "goblin-warrior uses Nimble Escape to Disengage");
    let goblin = out["digest"]["combatants"].as_array().unwrap().iter().find(|c| c["id"] == "goblin-warrior").unwrap();
    assert_eq!(goblin["action_economy"]["bonus_action_used"], true);
    assert_eq!(goblin["action_economy"]["action_used"], false);
    let replay = bench.ok(&["replay"]);
    assert_eq!(replay["matches"], true, "{replay:#}");
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
fn respond_take_answers_an_opportunity_attack_window() {
    let bench = Bench::new();
    bench.start(&[]);
    if bench.ok(&["state"])["digest"]["active_turn"] != "standard-striker-03" {
        bench.ok(&["end-turn"]);
    }
    bench.ok(&["move", "standard-striker-03", "engaged", "--target", "goblin-warrior"]);
    let held = bench.ok(&["move", "standard-striker-03", "near"]);
    assert_eq!(held["pending"]["kind"], "reaction");
    assert_eq!(held["pending"]["message"], "goblin-warrior: Opportunity Attack on standard-striker-03 (scimitar)?");
    assert_eq!(bench.err(&["respond", "4"]), "invalid_argument");
    assert_eq!(bench.err(&["respond", "--take", "shortbow"]), "invalid_argument");
    let out = bench.ok(&["respond", "--take"]);
    let lines: Vec<&str> = out["events"].as_array().unwrap().iter().map(|e| e["line"].as_str().unwrap()).collect();
    assert_eq!(lines[0], "PROMPT goblin-warrior: Opportunity Attack on standard-striker-03 (scimitar)? answered: takes Scimitar");
    assert_eq!(lines[1], "OPPORTUNITY ATTACK goblin-warrior on standard-striker-03 (reaction)");
    assert_eq!(bench.ok(&["replay"])["matches"], true);
    bench.ok(&["undo"]);
    bench.ok(&["respond", "--decline"]);
    assert_eq!(bench.ok(&["replay"])["matches"], true);
}

#[test]
fn end_turn_takes_declared_legendary_actions() {
    let bench = Bench::new();
    let mut goblin: Value = serde_json::from_str(&std::fs::read_to_string(data("fixtures/goblin-warrior.json")).unwrap()).unwrap();
    let mut kick = goblin["actions"][0].clone();
    kick["id"] = "kick".into();
    kick["name"] = "Kick".into();
    goblin["legendary_actions"] = Value::Array(vec![kick]);
    goblin["legendary_uses_per_round"] = 1.into();
    let path = bench.cwd.join("legendary-goblin.json");
    std::fs::write(&path, goblin.to_string()).unwrap();
    let pc = data("party/standard/standard-striker-03.json");
    bench.ok(&["start", pc.as_str(), path.to_str().unwrap(), "--seed", "11"]);
    if bench.ok(&["state"])["digest"]["active_turn"] != "standard-striker-03" {
        bench.ok(&["end-turn"]);
    }
    assert_eq!(bench.ok(&["state"])["digest"]["legendary"][0]["id"], "goblin-warrior");

    let out = bench.ok(&["end-turn", "--legendary", "goblin-warrior:kick=standard-striker-03"]);
    let lines: Vec<&str> = out["events"].as_array().unwrap().iter().map(|e| e["line"].as_str().unwrap()).collect();
    assert_eq!(lines[1], "LEGENDARY ACTION goblin-warrior: Kick (1 of 1)", "{lines:?}");
    assert_eq!(bench.err(&["end-turn", "--legendary", "goblin-warrior:kick=standard-striker-03"]), "illegal_action");
    for bad in ["goblin-warrior", "goblin-warrior:", ":kick", "goblin-warrior:kick=", "goblin-warrior:kick=a,"] {
        let out = Command::new(env!("CARGO_BIN_EXE_bench")).args(["end-turn", "--legendary", bad]).env("BENCH_HOME", &bench.home).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{bad}");
    }
    bench.ok(&["undo"]);
    bench.ok(&["end-turn", "--legendary", "goblin-warrior:kick=standard-striker-03"]);
    assert_eq!(bench.ok(&["replay"])["matches"], true);
}

#[test]
fn an_imported_cooldown_legendary_action_is_refused_a_second_time_in_one_round() {
    let bench = Bench::new();
    let pc = data("party/standard/standard-defender-20.json");
    let lich = data("fixtures/lich.json");
    bench.ok(&["start", pc.as_str(), lich.as_str(), "--seed", "11"]);
    if bench.ok(&["state"])["digest"]["active_turn"] != "standard-defender-20" {
        bench.ok(&["end-turn"]);
    }
    let disrupt = "lich:disrupt-life=standard-defender-20";
    let (code, json) = bench.run(&["end-turn", "--legendary", disrupt, "--legendary", disrupt]);
    assert_ne!(code, 0, "{json:#}");
    assert!(json["error"]["message"].as_str().unwrap().contains("disrupt-life is on cooldown"), "{json:#}");
    let out = bench.ok(&["end-turn", "--legendary", disrupt]);
    assert!(out["events"].as_array().unwrap().iter().any(|e| e["line"] == "LEGENDARY ACTION lich: Disrupt Life (1 of 3)"), "{out:#}");
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
        "start", "action", "move", "disengage", "respond", "end-turn", "end", "state", "list", "clean", "override", "roll", "add",
        "remove", "undo", "replay", "log", "batch", "fixture", "party", "baseline",
    ] {
        assert!(names.contains(&name), "{name} missing from {names:?}");
    }
}

#[test]
fn batch_is_byte_identical_and_fight_replays_its_log() {
    let bench = Bench::new();
    let args = ["batch", "ogre", "--seed", "5", "--levels", "2-3", "--fights", "3"];
    let run = |out: &str| {
        let mut a = args.to_vec();
        a.extend_from_slice(&["--out", out]);
        bench.ok(&a)
    };
    let first = run("one");
    let second = run("two");
    let html = |dir: &str| std::fs::read_to_string(bench.cwd.join(dir).join("report.html")).unwrap();
    assert_eq!(html("one"), html("two"));
    assert_eq!(first["levels"], second["levels"]);
    assert_eq!(first["master_seed"], 5);
    assert_eq!(first["levels"].as_array().unwrap().len(), 2);
    let sample = &first["samples"][0];
    let index = sample["index"].as_u64().unwrap().to_string();
    let replay = bench.ok(&["batch", "ogre", "--seed", "5", "--fights", "3", "--fight", index.as_str()]);
    let file = bench.cwd.join("one").join(sample["file"].as_str().unwrap());
    assert_eq!(replay["log"].as_str().unwrap(), std::fs::read_to_string(file).unwrap());
    assert!(replay["log"].as_str().unwrap().contains("STANDING ASSUMPTIONS"));
}

#[test]
fn batch_refuses_unresolved_homebrew_until_assumed() {
    let bench = Bench::new();
    let mut monster: Value = serde_json::from_str(&std::fs::read_to_string(data("fixtures/ogre.json")).unwrap()).unwrap();
    monster["id"] = "odd-ogre".into();
    monster["actions"][0]["agent_clauses"] = serde_json::json!(["It does something strange."]);
    let id = monster["actions"][0]["id"].as_str().unwrap().to_string();
    let path = bench.cwd.join("odd-ogre.json");
    std::fs::write(&path, monster.to_string()).unwrap();
    let path = path.display().to_string();
    let (code, json) = bench.run(&["batch", path.as_str(), "--levels", "1", "--fights", "1", "--seed", "1"]);
    assert_ne!(code, 0);
    assert_eq!(json["error"]["code"], "unmodeled_ability_refused");
    assert_eq!(json["error"]["details"]["abilities"][0], id.as_str());
    let assume = format!("abilities.{id}=ignored");
    let out = bench.ok(&["batch", path.as_str(), "--levels", "1", "--fights", "1", "--seed", "1", "--assume", assume.as_str(), "--out", "r"]);
    assert_eq!(out["standing_assumptions"][0]["path"], format!("abilities.{id}"));
}

fn shell_words(cmd: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut chars = cmd.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' => words.extend(word.take()),
            '\'' => {
                let w = word.get_or_insert_with(String::new);
                for q in chars.by_ref() {
                    if q == '\'' {
                        break;
                    }
                    w.push(q);
                }
            }
            '\\' => word.get_or_insert_with(String::new).extend(chars.next()),
            _ => word.get_or_insert_with(String::new).push(c),
        }
    }
    words.extend(word);
    words
}

#[test]
fn sample_replay_commands_rerun_homebrew_files_with_their_assumptions() {
    let bench = Bench::new();
    let mut monster: Value = serde_json::from_str(&std::fs::read_to_string(data("fixtures/ogre.json")).unwrap()).unwrap();
    monster["id"] = "odd-ogre".into();
    monster["actions"][0]["agent_clauses"] = serde_json::json!(["It does something strange."]);
    let id = monster["actions"][0]["id"].as_str().unwrap().to_string();
    std::fs::write(bench.cwd.join("my brew.json"), monster.to_string()).unwrap();
    let file_assumption = serde_json::json!([{ "path": "policy.weights.control", "value": 0.5, "reason": "file" }]);
    std::fs::write(bench.cwd.join("assume.json"), file_assumption.to_string()).unwrap();
    let assume = format!("abilities.{id}=ignored");
    let args = [
        "batch", "my brew.json", "--levels", "2", "--fights", "3", "--seed", "9", "--assumptions", "assume.json", "--assume",
        assume.as_str(), "--assume-reason", "the DM's call", "--out", "r",
    ];
    let out = bench.ok(&args);
    for sample in out["samples"].as_array().unwrap() {
        let replay = sample["replay"].as_str().unwrap();
        let words = shell_words(replay);
        assert_eq!(words[..2], ["bench", "batch"], "{replay}");
        let argv: Vec<&str> = words[1..].iter().map(String::as_str).collect();
        let fight = bench.ok(&argv);
        let file = bench.cwd.join("r").join(sample["file"].as_str().unwrap());
        assert_eq!(fight["log"].as_str().unwrap(), std::fs::read_to_string(file).unwrap(), "{replay}");
        assert_eq!(fight["replay"].as_str().unwrap(), replay);
    }
    let html = std::fs::read_to_string(bench.cwd.join("r/report.html")).unwrap();
    assert!(html.contains("my brew.json"));
}

#[test]
fn sample_replay_commands_keep_a_local_party_that_shadows_the_standard_name() {
    let bench = Bench::new();
    let dir = bench.cwd.join(".bench/parties/standard");
    std::fs::create_dir_all(&dir).unwrap();
    for file in ["standard-defender-02.json", "standard-striker-02.json"] {
        std::fs::copy(data(&format!("party/standard/{file}")), dir.join(file)).unwrap();
    }
    let out = bench.ok(&["batch", "ogre", "--party", "standard", "--levels", "2", "--fights", "2", "--seed", "3", "--out", "r"]);
    let sample = &out["samples"][0];
    let replay = sample["replay"].as_str().unwrap();
    assert!(replay.contains("--party standard --levels 2"), "{replay}");
    let words = shell_words(replay);
    let argv: Vec<&str> = words[1..].iter().map(String::as_str).collect();
    let fight = bench.ok(&argv);
    let file = bench.cwd.join("r").join(sample["file"].as_str().unwrap());
    assert_eq!(fight["log"].as_str().unwrap(), std::fs::read_to_string(file).unwrap());
}

#[test]
fn batch_argument_errors() {
    let bench = Bench::new();
    assert_eq!(bench.err(&["batch", "no-such-monster", "--seed", "1"]), "not_found");
    assert_eq!(bench.err(&["batch", "ogre", "--levels", "0-3", "--seed", "1"]), "invalid_argument");
    assert_eq!(bench.err(&["batch", "ogre", "--party", "missing", "--seed", "1"]), "not_found");
}

#[test]
fn fixture_and_party_commands_print_embedded_data() {
    let bench = Bench::new();
    let list = bench.ok(&["fixture", "list"]);
    assert!(list["count"].as_u64().unwrap() >= 341);
    let ogre = list["fixtures"].as_array().unwrap().iter().find(|f| f["id"] == "ogre").unwrap();
    assert_eq!(ogre["challenge_rating"], 2.0);
    let shown = bench.ok(&["fixture", "show", "ogre"]);
    assert_eq!(shown["id"], "ogre");
    assert_eq!(bench.err(&["fixture", "show", "no-such-monster"]), "not_found");
    let level = bench.ok(&["party", "show", "--level", "3"]);
    assert_eq!(level["members"].as_array().unwrap().len(), 4);
    assert_eq!(level["members"][0]["id"], "standard-defender-03");
    assert_eq!(bench.ok(&["party", "show"])["levels"].as_object().unwrap().len(), 20);
}

#[test]
fn bare_fixture_slugs_start_an_encounter_and_files_still_win() {
    let bench = Bench::new();
    let out = bench.ok(&["start", "standard-striker-03", "goblin-warrior", "--seed", "3"]);
    assert!(out.to_string().contains("goblin-warrior"));
    let path = bench.cwd.join("ogre");
    std::fs::write(&path, "not json").unwrap();
    assert_eq!(bench.err(&["start", "ogre", "goblin-warrior"]), "invalid_json");
}

#[test]
fn batch_compares_against_the_embedded_curve_outside_the_repo() {
    let bench = Bench::new();
    let out = bench.ok(&["batch", "ogre", "--seed", "5", "--levels", "2-3", "--fights", "3", "--out", "r"]);
    let comparison = &out["comparison"];
    assert!(comparison["verdict"].as_str().unwrap().contains("CR 2"));
    assert!(comparison["basis"].as_str().unwrap().starts_with("Basis: "));
    assert!(out["srd"]["xp"] == 450);
    let status = bench.ok(&["baseline", "status"]);
    assert!(status["reasons"].as_array().unwrap().is_empty(), "the embedded curve is stale: {status:#}");
    let html = std::fs::read_to_string(bench.cwd.join("r/report.html")).unwrap();
    assert!(html.contains("Claimed-CR verdict"));
}

#[test]
fn lighting_is_a_start_flag_and_a_misplaced_roll_modifier_fails_at_load() {
    let bench = Bench::new();
    let out = bench.start(&["--lighting", "bright_light"]);
    assert_eq!(out["digest"]["lighting"], "bright_light");
    bench.ok(&["override", "settings.lighting", "sunlight", "--reason", "noon"]);
    assert_eq!(bench.ok(&["state", "--public"])["digest"]["lighting"], "sunlight");

    let mut monster: Value = serde_json::from_str(&std::fs::read_to_string(data("fixtures/kobold-warrior.json")).unwrap()).unwrap();
    monster["id"] = "glaring-kobold".into();
    monster["actions"][0]["effects"][0]["attack"]["on_hit"] =
        serde_json::json!([{"roll_modifier": {"lighting": "sunlight", "disposition": "disadvantage", "target": "attack_rolls"}}]);
    let path = bench.cwd.join("glaring-kobold.json");
    std::fs::write(&path, monster.to_string()).unwrap();
    let path = path.display().to_string();
    for args in [vec!["start", path.as_str(), "--seed", "1"], vec!["batch", path.as_str(), "--levels", "1", "--fights", "1", "--seed", "1"]] {
        let (code, json) = bench.run(&args);
        assert_ne!(code, 0);
        assert_eq!(json["error"]["details"][0]["message"].as_str().map(|m| m.starts_with("dagger: a roll modifier is a trait effect")), Some(true), "{json:#}");
    }
    assert_eq!(bench.ok(&["list"])["encounters"].as_array().unwrap().len(), 1);
}

#[test]
fn a_cast_below_its_spell_level_is_refused_before_any_dice() {
    let bench = Bench::new();
    let mut monster: Value = serde_json::from_str(&std::fs::read_to_string(data("fixtures/kobold-warrior.json")).unwrap()).unwrap();
    monster["id"] = "fiery-kobold".into();
    monster["actions"].as_array_mut().unwrap().push(serde_json::json!({
        "id": "fireball", "name": "Fireball", "kind": "modeled",
        "effects": [{"cast": {"spell_id": "fireball", "slot_level": 1, "upcast": false}}]
    }));
    let path = bench.cwd.join("fiery-kobold.json");
    std::fs::write(&path, monster.to_string()).unwrap();
    let path = path.display().to_string();
    for args in [vec!["start", path.as_str(), "--seed", "1"], vec!["batch", path.as_str(), "--levels", "1", "--fights", "1", "--seed", "1"]] {
        let (code, json) = bench.run(&args);
        assert_ne!(code, 0);
        let message = json["error"]["details"][0]["message"].as_str().unwrap_or_default();
        assert!(message.starts_with("fireball: a cast's slot level is below the spell's level"), "{json:#}");
    }
    assert_eq!(bench.ok(&["list"])["encounters"].as_array().unwrap().len(), 0);
}

#[test]
fn schema_validate_warns_on_a_self_buff_with_no_bearer() {
    let bench = Bench::new();
    let knight = data("fixtures/knight.json");
    assert!(bench.ok(&["schema", "validate", knight.as_str()]).get("warnings").is_none());
    let mut block: Value = serde_json::from_str(&std::fs::read_to_string(&knight).unwrap()).unwrap();
    let parry = &mut block["reactions"][0]["effects"][0]["timed_modifier"];
    parry.as_object_mut().unwrap().remove("bearer");
    parry["duration"] = serde_json::json!({"rounds": 1});
    let path = bench.cwd.join("braced-knight.json");
    std::fs::write(&path, block.to_string()).unwrap();
    let data = bench.ok(&["schema", "validate", path.to_str().unwrap()]);
    assert_eq!(data["valid"], true);
    let warnings = data["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{data:#}");
    assert!(warnings[0].as_str().unwrap().contains("add \"bearer\": \"self\""));
}
