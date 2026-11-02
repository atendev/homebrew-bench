#!/bin/sh
# Install or update `bench`, its legal notices, and the homebrew-bench agent skill.
#
#   curl -fsSL https://raw.githubusercontent.com/atendev/homebrew-bench/master/install.sh | sh
#   ./install.sh [--no-skills] [--agent <name>]... [--version <tag>] [--from-source]
#
# Environment overrides: BENCH_HOME (default ~/.bench), BENCH_REPO (owner/name),
# BENCH_RELEASES_URL (release download base), BENCH_GIT_URL (source to clone),
# BENCH_TARGET (target triple).

set -eu

REPO="${BENCH_REPO:-atendev/homebrew-bench}"
RELEASES="${BENCH_RELEASES_URL:-https://github.com/$REPO/releases}"
GIT_URL="${BENCH_GIT_URL:-https://github.com/$REPO.git}"
SKILL=homebrew-bench

usage() {
    cat <<'EOF'
Usage: install.sh [options]

Installs bench into ~/.bench/bin, with NOTICE, LICENSE and the homebrew-bench
skill, then copies the skill into each agent's user skill folder. Re-running
updates the install and leaves config.toml, index, parties/, house-rules/,
encounters/ and reports/ alone.

Options:
  --no-skills          Install no agent skills outside ~/.bench.
  --agent <name>       Install the skill for this agent only: claude, cline,
                       kiro, agents (~/.agents/skills) or all. Repeatable.
                       Without it, the skill goes to ~/.agents/skills plus
                       each of claude, cline and kiro found on PATH or with a
                       ~/.<agent> folder.
  --version <tag>      Release to install (default: latest).
  --from-source        Build with cargo instead of downloading a release.
                       Inside a checkout this builds the checkout, which is
                       also the default when --version is not given.
  -h, --help           Show this help.
EOF
}

say() { printf '%s\n' "$*"; }
warn() { printf 'install.sh: %s\n' "$*" >&2; }
die() { warn "$*"; exit 1; }
usage_error() { warn "$*"; usage >&2; exit 2; }

no_skills=0
agents=""
version=""
from_source=0

add_agent() {
    case "$1" in
        claude | cline | kiro | agents) agents="$agents $1" ;;
        all) agents="$agents agents claude cline kiro" ;;
        *) usage_error "unknown agent '$1': use claude, cline, kiro, agents or all" ;;
    esac
}

while [ $# -gt 0 ]; do
    case "$1" in
        --no-skills) no_skills=1 ;;
        --agent)
            [ $# -ge 2 ] || usage_error "--agent needs a value"
            add_agent "$2"
            shift
            ;;
        --agent=*) add_agent "${1#--agent=}" ;;
        --version)
            [ $# -ge 2 ] || usage_error "--version needs a value"
            version="$2"
            shift
            ;;
        --version=*) version="${1#--version=}" ;;
        --from-source) from_source=1 ;;
        -h | --help)
            usage
            exit 0
            ;;
        *) usage_error "unknown option '$1' (see --help)" ;;
    esac
    shift
done

[ -n "$version" ] || [ "$from_source" = 1 ] || version_default=1
[ "$no_skills" = 0 ] || [ -z "$agents" ] || usage_error "--no-skills and --agent cannot be combined"
[ -n "${HOME:-}" ] || die "HOME is not set"

BENCH_HOME="${BENCH_HOME:-$HOME/.bench}"

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || script_dir=""
checkout=""
if [ -f "$0" ] && [ -n "$script_dir" ] && [ -f "$script_dir/install.sh" ] && [ -f "$script_dir/bench/Cargo.toml" ] &&
    [ -d "$script_dir/.agents/skills/$SKILL" ]; then
    checkout="$script_dir"
fi

if [ "$from_source" = 1 ] && [ -n "$checkout" ] && [ -n "$version" ]; then
    usage_error "--version picks a release, but --from-source inside a checkout builds the checkout; drop one"
fi

tmp=$(mktemp -d 2>/dev/null || mktemp -d -t bench-install)
trap 'rm -rf "$tmp"' EXIT INT TERM

stage_bin=""
stage_dir=""
stage_skill=""

fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$2" "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$2" "$1"
    else
        die "need curl or wget to download a release"
    fi
}

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d' ' -f1
    else
        echo ""
    fi
}

detect_target() {
    if [ -n "${BENCH_TARGET:-}" ]; then
        echo "$BENCH_TARGET"
        return
    fi
    case "$(uname -s)" in
        Linux) os=unknown-linux-gnu ;;
        Darwin) os=apple-darwin ;;
        *) return 1 ;;
    esac
    case "$(uname -m)" in
        x86_64 | amd64) arch=x86_64 ;;
        aarch64 | arm64) arch=aarch64 ;;
        *) return 1 ;;
    esac
    echo "$arch-$os"
}

find_skill_dir() {
    for candidate in "$1/$SKILL" "$1/skills/$SKILL" "$1/.agents/skills/$SKILL"; do
        if [ -f "$candidate/SKILL.md" ]; then
            echo "$candidate"
            return
        fi
    done
    return 1
}

# Download and unpack a release archive. Returns 1 when no prebuilt archive
# exists for this platform or tag, so the caller can fall back to a build.
stage_release() {
    target=$(detect_target) || {
        warn "no prebuilt bench for $(uname -s) $(uname -m)"
        return 1
    }
    archive="bench-$target.tar.gz"
    if [ "${version:-latest}" = latest ]; then
        url="$RELEASES/latest/download/$archive"
    else
        url="$RELEASES/download/$version/$archive"
    fi
    say "Downloading $url"
    fetch "$url" "$tmp/$archive" 2>/dev/null || {
        warn "no release archive at $url"
        return 1
    }
    if fetch "$url.sha256" "$tmp/$archive.sha256" 2>/dev/null; then
        want=$(cut -d' ' -f1 <"$tmp/$archive.sha256")
        have=$(sha256_of "$tmp/$archive")
        if [ -n "$have" ] && [ "$have" != "$want" ]; then
            die "checksum mismatch for $archive: expected $want, got $have"
        fi
    fi
    mkdir -p "$tmp/release"
    tar -xzf "$tmp/$archive" -C "$tmp/release" || die "could not unpack $archive"
    stage_bin=$(find "$tmp/release" -maxdepth 3 -type f -name bench | head -n 1)
    [ -n "$stage_bin" ] || die "$archive has no bench binary"
    stage_dir=$(dirname -- "$stage_bin")
    stage_skill=$(find_skill_dir "$stage_dir") || die "$archive has no $SKILL skill"
}

stage_source() {
    src="$checkout"
    if [ -z "$src" ]; then
        command -v git >/dev/null 2>&1 || die "need git to fetch the source"
        src="$tmp/src"
        say "Cloning $GIT_URL"
        if [ -n "$version" ] && [ "$version" != latest ]; then
            git clone --quiet --depth 1 --branch "$version" "$GIT_URL" "$src" || die "could not clone $version from $GIT_URL"
        else
            git clone --quiet --depth 1 "$GIT_URL" "$src" || die "could not clone $GIT_URL"
        fi
    fi
    command -v cargo >/dev/null 2>&1 || die "need cargo to build from source; install Rust from https://rustup.rs"
    say "Building bench in $src"
    (cd "$src" && cargo build --release --locked -p bench) || die "cargo build failed"
    target_dir="${CARGO_TARGET_DIR:-target}"
    case "$target_dir" in
        /*) ;;
        *) target_dir="$src/$target_dir" ;;
    esac
    stage_bin="$target_dir/release/bench"
    [ -f "$stage_bin" ] || die "cargo build did not produce $stage_bin"
    stage_dir="$src"
    stage_skill="$src/.agents/skills/$SKILL"
}

replace_dir() {
    rm -rf "$2.new"
    mkdir -p "$(dirname -- "$2")"
    cp -RL "$1" "$2.new"
    rm -rf "$2"
    mv "$2.new" "$2"
}

if [ "$from_source" = 1 ] || { [ -n "$checkout" ] && [ -n "${version_default:-}" ]; }; then
    stage_source
elif ! stage_release; then
    # A release was asked for, so build that release's source, not the checkout.
    warn "building ${version:-latest} from source instead"
    checkout=""
    stage_source
fi

for file in NOTICE LICENSE; do
    [ -f "$stage_dir/$file" ] || die "missing $file next to the bench binary"
done

mkdir -p "$BENCH_HOME/bin"
cp "$stage_bin" "$BENCH_HOME/bin/bench.new"
chmod 755 "$BENCH_HOME/bin/bench.new"
mv -f "$BENCH_HOME/bin/bench.new" "$BENCH_HOME/bin/bench"
cp "$stage_dir/NOTICE" "$stage_dir/LICENSE" "$BENCH_HOME/"
replace_dir "$stage_skill" "$BENCH_HOME/skills/$SKILL"

if [ ! -e "$BENCH_HOME/config.toml" ]; then
    cat >"$BENCH_HOME/config.toml" <<'EOF'
# bench settings. See the homebrew-bench skill for what each one does.
output = "home"
keep_encounters = true
delete_logs = false
round_cap = 20
EOF
fi
[ -e "$BENCH_HOME/index" ] || printf '{\n  "encounters": []\n}\n' >"$BENCH_HOME/index"

say "Installed bench to $BENCH_HOME/bin/bench"

if [ "$no_skills" = 0 ]; then
    if [ -z "$agents" ]; then
        agents="agents"
        for agent in claude cline kiro; do
            if command -v "$agent" >/dev/null 2>&1 || [ -d "$HOME/.$agent" ]; then
                agents="$agents $agent"
            fi
        done
    fi
    seen=" "
    for agent in $agents; do
        case "$seen" in *" $agent "*) continue ;; esac
        seen="$seen$agent "
        dest="$HOME/.$agent/skills/$SKILL"
        replace_dir "$BENCH_HOME/skills/$SKILL" "$dest"
        say "Installed the $SKILL skill to $dest"
    done
fi

case ":${PATH:-}:" in
    *":$BENCH_HOME/bin:"*) ;;
    *) say "Add bench to your PATH: export PATH=\"$BENCH_HOME/bin:\$PATH\"" ;;
esac
