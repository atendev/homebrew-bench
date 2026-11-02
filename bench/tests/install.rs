#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const TARGET: &str = "test-triple";
const TOOLS: &[&str] = &[
    "sh", "tar", "gzip", "curl", "mkdir", "cp", "rm", "mv", "chmod", "find", "head", "dirname", "cut", "sha256sum",
    "mktemp", "uname", "cat", "git",
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn write_exe(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A temporary HOME, a PATH holding only the tools the installer needs, and a
/// file:// release server with no releases yet.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("bench-install-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["home", "tools", "extra-path", "releases", "cargo-target"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for tool in TOOLS {
            let found = ["/usr/bin", "/bin", "/usr/local/bin"].iter().map(|d| Path::new(d).join(tool)).find(|p| p.exists());
            let found = found.unwrap_or_else(|| panic!("installer tests need `{tool}`"));
            std::os::unix::fs::symlink(found, root.join("tools").join(tool)).unwrap();
        }
        Self { root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn bench_home(&self) -> PathBuf {
        self.home().join(".bench")
    }

    /// Puts a fake agent binary on PATH.
    fn agent_on_path(&self, name: &str) {
        write_exe(&self.root.join("extra-path").join(name), "#!/bin/sh\n");
    }

    /// Publishes a release archive under `tag` whose `bench` prints `marker`.
    fn publish(&self, tag: &str, marker: &str) {
        let dir = if tag == "latest" { self.root.join("releases/latest/download") } else { self.root.join("releases/download").join(tag) };
        let name = format!("bench-{TARGET}");
        let stage = self.root.join("stage").join(tag).join(&name);
        write_exe(&stage.join("bench"), &format!("#!/bin/sh\necho {marker}\n"));
        std::fs::write(stage.join("NOTICE"), format!("notice {marker}\n")).unwrap();
        std::fs::write(stage.join("LICENSE"), "license\n").unwrap();
        std::fs::create_dir_all(stage.join("homebrew-bench/references")).unwrap();
        std::fs::write(stage.join("homebrew-bench/SKILL.md"), format!("skill {marker}\n")).unwrap();
        std::fs::write(stage.join("homebrew-bench/references/live-mode.md"), "live\n").unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join(format!("{name}.tar.gz"));
        let status = Command::new("tar").arg("-czf").arg(&archive).arg("-C").arg(stage.parent().unwrap()).arg(&name).status().unwrap();
        assert!(status.success());
        let sum = Command::new("sha256sum").arg(&archive).output().unwrap();
        let hash = String::from_utf8(sum.stdout).unwrap().split_whitespace().next().unwrap().to_string();
        std::fs::write(dir.join(format!("{name}.tar.gz.sha256")), format!("{hash} *{name}.tar.gz\n")).unwrap();
    }

    /// Puts a fake `cargo` on PATH that logs its arguments and "builds" a stub binary.
    fn fake_cargo(&self) {
        let script = format!(
            "#!/bin/sh\necho \"$@\" >> '{log}'\nmkdir -p \"$CARGO_TARGET_DIR/release\"\nprintf '#!/bin/sh\\necho built-from-source\\n' > \"$CARGO_TARGET_DIR/release/bench\"\nchmod 755 \"$CARGO_TARGET_DIR/release/bench\"\n",
            log = self.root.join("cargo.log").display()
        );
        write_exe(&self.root.join("extra-path/cargo"), &script);
    }

    fn cargo_log(&self) -> String {
        std::fs::read_to_string(self.root.join("cargo.log")).unwrap_or_default()
    }

    /// Commits a minimal source tree tagged `tag` and returns its git URL.
    fn publish_source(&self, tag: &str) -> String {
        let src = self.root.join("upstream");
        std::fs::create_dir_all(src.join("bench")).unwrap();
        std::fs::create_dir_all(src.join(".agents/skills/homebrew-bench")).unwrap();
        std::fs::write(src.join("bench/Cargo.toml"), "").unwrap();
        std::fs::write(src.join("NOTICE"), "notice from clone\n").unwrap();
        std::fs::write(src.join("LICENSE"), "license\n").unwrap();
        std::fs::write(src.join(".agents/skills/homebrew-bench/SKILL.md"), "skill from clone\n").unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git").args(args).current_dir(&src).output().unwrap();
            assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        };
        git(&["init", "--quiet"]);
        git(&["add", "."]);
        git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "--quiet", "-m", "src"]);
        git(&["tag", tag]);
        format!("file://{}", src.display())
    }

    fn command(&self, script: &str) -> Command {
        let path = format!("{}:{}", self.root.join("extra-path").display(), self.root.join("tools").display());
        let mut cmd = Command::new(self.root.join("tools/sh"));
        cmd.arg(repo().join(script))
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", path)
            .env("BENCH_RELEASES_URL", format!("file://{}", self.root.join("releases").display()))
            .env("BENCH_GIT_URL", format!("file://{}", self.root.join("no-such-repo").display()))
            .env("BENCH_TARGET", TARGET)
            .env("CARGO_TARGET_DIR", self.root.join("cargo-target"));
        cmd
    }

    fn run_script(&self, script: &str, args: &[&str]) -> Output {
        self.command(script).args(args).output().unwrap()
    }

    fn install(&self, args: &[&str]) -> Output {
        let out = self.run_script("install.sh", args);
        assert!(out.status.success(), "install.sh {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
        out
    }

    fn install_err(&self, code: i32, args: &[&str]) -> String {
        let out = self.run_script("install.sh", args);
        assert_eq!(out.status.code(), Some(code), "install.sh {args:?} exit code");
        String::from_utf8(out.stderr).unwrap()
    }

    fn installed_bench(&self) -> String {
        let out = Command::new(self.bench_home().join("bin/bench")).output().unwrap();
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    fn skill_targets(&self) -> Vec<&'static str> {
        ["agents", "claude", "cline", "kiro"]
            .into_iter()
            .filter(|a| self.home().join(format!(".{a}/skills/homebrew-bench/SKILL.md")).is_file())
            .collect()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn read(path: impl AsRef<Path>) -> String {
    std::fs::read_to_string(path.as_ref()).unwrap_or_else(|e| panic!("{}: {e}", path.as_ref().display()))
}

#[test]
fn flags_are_parsed_and_checked() {
    let sb = Sandbox::new();
    let help = sb.run_script("install.sh", &["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    for flag in ["--no-skills", "--agent <name>", "--version <tag>", "--from-source"] {
        assert!(text.contains(flag), "help lacks {flag}");
    }
    let bogus = sb.install_err(2, &["--bogus"]);
    assert!(bogus.contains("unknown option '--bogus'") && bogus.contains("Usage:"));
    assert!(sb.install_err(2, &["--agent", "vim"]).contains("unknown agent 'vim'"));
    assert!(sb.install_err(2, &["--agent"]).contains("--agent needs a value"));
    assert!(sb.install_err(2, &["--version"]).contains("--version needs a value"));
    assert!(sb.install_err(2, &["--no-skills", "--agent", "claude"]).contains("cannot be combined"));
    assert!(sb.install_err(2, &["--from-source", "--version", "v1"]).contains("drop one"));
    assert!(!sb.bench_home().exists(), "a rejected command wrote files");
}

#[test]
fn release_install_lays_out_bench_home() {
    let sb = Sandbox::new();
    sb.publish("latest", "release-latest");
    sb.fake_cargo();
    sb.install(&["--version", "latest"]);

    let home = sb.bench_home();
    assert_eq!(sb.installed_bench(), "release-latest");
    assert_eq!(read(home.join("NOTICE")), "notice release-latest\n");
    assert_eq!(read(home.join("LICENSE")), "license\n");
    assert_eq!(read(home.join("skills/homebrew-bench/SKILL.md")), "skill release-latest\n");
    assert!(home.join("skills/homebrew-bench/references/live-mode.md").is_file());
    assert_eq!(sb.cargo_log(), "", "a release install ran cargo");

    let list = Command::new(env!("CARGO_BIN_EXE_bench")).arg("list").env("BENCH_HOME", &home).output().unwrap();
    assert!(list.status.success(), "the real bench rejects the installed config or index: {}", String::from_utf8_lossy(&list.stdout));
    let config = bench_core::config::Config::load(home.join("config.toml")).unwrap();
    assert_eq!(format!("{config:?}"), format!("{:?}", bench_core::config::Config::default()));

    assert_eq!(sb.skill_targets(), ["agents"]);
    let copy = sb.home().join(".agents/skills/homebrew-bench");
    assert!(!std::fs::symlink_metadata(&copy).unwrap().file_type().is_symlink());
}

#[test]
fn named_tag_and_checksum() {
    let sb = Sandbox::new();
    sb.publish("v1.2.3", "tagged");
    sb.install(&["--version", "v1.2.3", "--no-skills"]);
    assert_eq!(sb.installed_bench(), "tagged");

    let sb = Sandbox::new();
    sb.publish("v1", "tampered");
    std::fs::write(sb.root.join(format!("releases/download/v1/bench-{TARGET}.tar.gz.sha256")), "0000 *x\n").unwrap();
    assert!(sb.install_err(1, &["--version", "v1"]).contains("checksum mismatch"));
    assert!(!sb.bench_home().join("bin/bench").exists());
}

#[test]
fn skill_targets_follow_detection_and_agent_flags() {
    let sb = Sandbox::new();
    sb.publish("latest", "r");
    std::fs::create_dir_all(sb.home().join(".kiro")).unwrap();
    sb.agent_on_path("claude");
    sb.install(&["--version", "latest"]);
    assert_eq!(sb.skill_targets(), ["agents", "claude", "kiro"]);

    let sb = Sandbox::new();
    sb.publish("latest", "r");
    std::fs::create_dir_all(sb.home().join(".kiro")).unwrap();
    sb.install(&["--version", "latest", "--agent", "claude"]);
    assert_eq!(sb.skill_targets(), ["claude"]);

    let sb = Sandbox::new();
    sb.publish("latest", "r");
    sb.install(&["--version", "latest", "--agent", "cline", "--agent=agents"]);
    assert_eq!(sb.skill_targets(), ["agents", "cline"]);

    let sb = Sandbox::new();
    sb.publish("latest", "r");
    sb.install(&["--version", "latest", "--agent", "all"]);
    assert_eq!(sb.skill_targets(), ["agents", "claude", "cline", "kiro"]);

    let sb = Sandbox::new();
    sb.publish("latest", "r");
    std::fs::create_dir_all(sb.home().join(".claude")).unwrap();
    sb.install(&["--version", "latest", "--no-skills"]);
    assert!(sb.skill_targets().is_empty());
    assert!(sb.bench_home().join("skills/homebrew-bench/SKILL.md").is_file());
}

#[test]
fn rerun_updates_bench_and_skills_but_keeps_user_files() {
    let sb = Sandbox::new();
    sb.publish("v1", "one");
    sb.publish("v2", "two");
    sb.install(&["--version", "v1"]);

    let home = sb.bench_home();
    let user_files = [
        ("config.toml", "output = \"workspace\"\n"),
        ("index", "{\"encounters\": []}\n"),
        ("parties/mine/a.json", "{}"),
        ("house-rules/flanking.md", "rule"),
        ("encounters/goblin-1/state.json", "{}"),
        ("reports/r/report.html", "<p>"),
    ];
    for (rel, body) in user_files {
        std::fs::create_dir_all(home.join(rel).parent().unwrap()).unwrap();
        std::fs::write(home.join(rel), body).unwrap();
    }
    std::fs::write(home.join("skills/homebrew-bench/stale.md"), "old").unwrap();
    std::fs::write(sb.home().join(".agents/skills/homebrew-bench/stale.md"), "old").unwrap();

    sb.install(&["--version", "v2"]);
    assert_eq!(sb.installed_bench(), "two");
    assert_eq!(read(home.join("NOTICE")), "notice two\n");
    assert_eq!(read(sb.home().join(".agents/skills/homebrew-bench/SKILL.md")), "skill two\n");
    assert!(!home.join("skills/homebrew-bench/stale.md").exists());
    assert!(!sb.home().join(".agents/skills/homebrew-bench/stale.md").exists());
    for (rel, body) in user_files {
        assert_eq!(read(home.join(rel)), body, "{rel} changed on re-run");
    }
}

#[test]
fn from_source_builds_the_checkout() {
    let sb = Sandbox::new();
    sb.fake_cargo();
    sb.install(&["--from-source", "--agent", "agents"]);
    assert_eq!(sb.cargo_log().trim(), "build --release --locked -p bench");
    assert_eq!(sb.installed_bench(), "built-from-source");
    assert_eq!(read(sb.bench_home().join("NOTICE")), read(repo().join("NOTICE")));
    assert_eq!(read(sb.bench_home().join("LICENSE")), read(repo().join("LICENSE")));
    let skill = repo().join(".agents/skills/homebrew-bench/SKILL.md");
    assert_eq!(read(sb.bench_home().join("skills/homebrew-bench/SKILL.md")), read(&skill));
    assert_eq!(read(sb.home().join(".agents/skills/homebrew-bench/SKILL.md")), read(&skill));
}

#[test]
fn checkout_without_version_builds_from_source() {
    let sb = Sandbox::new();
    sb.publish("latest", "release");
    sb.fake_cargo();
    sb.install(&["--no-skills"]);
    assert_eq!(sb.installed_bench(), "built-from-source");
}

#[test]
fn missing_release_builds_that_tag_from_a_clone() {
    let sb = Sandbox::new();
    sb.fake_cargo();
    let url = sb.publish_source("v9.9.9");
    let out = sb.command("install.sh").args(["--version", "v9.9.9", "--no-skills"]).env("BENCH_GIT_URL", &url).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8(out.stderr).unwrap().contains("building v9.9.9 from source instead"));
    assert_eq!(sb.installed_bench(), "built-from-source");
    assert_eq!(read(sb.bench_home().join("NOTICE")), "notice from clone\n", "built the local checkout, not the tag");
    assert_eq!(read(sb.bench_home().join("skills/homebrew-bench/SKILL.md")), "skill from clone\n");

    let out = sb.command("install.sh").args(["--version", "v0.0.0", "--no-skills"]).env("BENCH_GIT_URL", &url).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr).unwrap().contains("could not clone v0.0.0"));
}

#[test]
fn piped_install_inside_a_clone_downloads_the_release() {
    let sb = Sandbox::new();
    sb.publish("latest", "piped");
    sb.fake_cargo();
    let template = sb.command("install.sh");
    let envs: Vec<_> = template.get_envs().filter_map(|(k, v)| v.map(|v| (k.to_owned(), v.to_owned()))).collect();
    let mut child = Command::new(sb.root.join("tools/sh"))
        .args(["-s", "--", "--no-skills"])
        .env_clear()
        .envs(envs)
        .current_dir(repo())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(read(repo().join("install.sh")).as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(sb.installed_bench(), "piped");
    assert_eq!(sb.cargo_log(), "");
}

#[test]
fn update_reruns_the_installer() {
    let sb = Sandbox::new();
    sb.publish("v1", "one");
    sb.publish("v2", "two");
    sb.install(&["--version", "v1"]);
    std::fs::write(sb.bench_home().join("config.toml"), "round_cap = 5\n").unwrap();
    let out = sb.run_script("update.sh", &["--version", "v2"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(sb.installed_bench(), "two");
    assert_eq!(read(sb.bench_home().join("config.toml")), "round_cap = 5\n");
}

/// Builds the real binary with cargo into a fresh target directory. Slow, so
/// run it with `cargo test --test install -- --ignored`.
#[test]
#[ignore]
fn from_source_with_real_cargo() {
    let sb = Sandbox::new();
    let cargo = Command::new("sh").args(["-c", "command -v cargo"]).output().unwrap();
    let cargo = String::from_utf8(cargo.stdout).unwrap();
    std::os::unix::fs::symlink(cargo.trim(), sb.root.join("extra-path/cargo")).unwrap();
    let real_home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let cargo_home = std::env::var_os("CARGO_HOME").map(PathBuf::from).unwrap_or(real_home.join(".cargo"));
    let rustup_home = std::env::var_os("RUSTUP_HOME").map(PathBuf::from).unwrap_or(real_home.join(".rustup"));
    let out = Command::new("sh")
        .arg(repo().join("install.sh"))
        .args(["--from-source", "--no-skills"])
        .env("HOME", sb.home())
        .env("CARGO_HOME", cargo_home)
        .env("RUSTUP_HOME", rustup_home)
        .env("CARGO_TARGET_DIR", sb.root.join("cargo-target"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let help = Command::new(sb.bench_home().join("bin/bench")).args(["help", "--json"]).output().unwrap();
    assert!(help.status.success());
}
