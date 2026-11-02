#!/bin/sh
# Undo install.sh and install-wizard.sh: remove bench, its notices and skill
# copies, and the PATH line the installer or wizard added to your shell
# profile. Your data (config, index, parties, house-rules, encounters, reports)
# stays unless you pass --remove-data. Project-local ./.bench folders are never
# touched.
#
#   sh uninstall.sh [--yes] [--remove-data] [--dry-run]
#
# Environment override: BENCH_HOME (default ~/.bench).

set -eu

SKILL=homebrew-bench
YES=0
REMOVE_DATA=0
DRY_RUN=0

usage() {
    cat <<'EOF'
Usage: uninstall.sh [options]

Removes bench from ~/.bench/bin with its NOTICE, LICENSE and skill copy, the
homebrew-bench skill in ~/.agents, ~/.claude, ~/.cline and ~/.kiro, and the
PATH line in your shell profile.

Options:
  --yes            Skip the questions. Your data is still kept unless
                   --remove-data is also given.
  --remove-data    Also remove config.toml, index, parties/, house-rules/,
                   encounters/ and reports/. Asks first unless --yes.
  --dry-run        Print what would be removed and change nothing.
  -h, --help       Show this help.
EOF
}

say() { printf '%s\n' "$*"; }
die() { printf 'uninstall: %s\n' "$*" >&2; exit 1; }

for arg in "$@"; do
    case "$arg" in
        --yes) YES=1 ;;
        --remove-data) REMOVE_DATA=1 ;;
        --dry-run) DRY_RUN=1 ;;
        -h|--help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done

BENCH_HOME="${BENCH_HOME:-$HOME/.bench}"
case "$BENCH_HOME" in
    ""|/|"$HOME") die "refusing to remove BENCH_HOME=$BENCH_HOME" ;;
esac

# confirm "question": y/N gate. Reads from the terminal, so it works when the
# script is run from a file.
confirm() {
    [ "$YES" = 1 ] && return 0
    [ -r /dev/tty ] || die "no terminal to ask on; re-run with --yes"
    printf '%s [y/N] ' "$1" >/dev/tty
    reply=""
    read -r reply </dev/tty || true
    case "$reply" in
        y|Y|yes|YES) return 0 ;;
    esac
    return 1
}

# Paths to delete, one per line. Only paths that exist are listed.
targets=""
add() {
    if [ -e "$1" ]; then
        targets="$targets$1
"
    fi
}

# Skill copies: remove only folders whose SKILL.md is ours, so a different
# skill that happens to share the folder name is left alone.
for agent_dir in "$HOME/.agents" "$HOME/.claude" "$HOME/.cline" "$HOME/.kiro"; do
    skill_dir="$agent_dir/skills/$SKILL"
    if [ -f "$skill_dir/SKILL.md" ] && grep -q "^name: $SKILL\$" "$skill_dir/SKILL.md"; then
        add "$skill_dir"
    fi
done

add "$BENCH_HOME/bin/bench"
add "$BENCH_HOME/NOTICE"
add "$BENCH_HOME/LICENSE"
add "$BENCH_HOME/skills/$SKILL"

data=""
for item in config.toml index parties house-rules encounters reports; do
    if [ -e "$BENCH_HOME/$item" ]; then
        data="$data$BENCH_HOME/$item
"
    fi
done

# The PATH line both installers and the wizard write, matched exactly.
path_line="export PATH=\"$BENCH_HOME/bin:\$PATH\""
profiles=""
for profile in "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.profile"; do
    if [ -f "$profile" ] && grep -qFx "$path_line" "$profile"; then
        profiles="$profiles$profile
"
    fi
done

if [ -z "$targets" ] && [ -z "$profiles" ] && { [ -z "$data" ] || [ "$REMOVE_DATA" = 0 ]; }; then
    say "Nothing to remove. bench is not installed in $BENCH_HOME."
    exit 0
fi

say "bench will be removed:"
printf '%s' "$targets" | while IFS= read -r p; do
    [ -n "$p" ] && say "  remove $p"
done
printf '%s' "$profiles" | while IFS= read -r p; do
    [ -n "$p" ] && say "  edit   $p (delete the line: $path_line)"
done
if [ "$REMOVE_DATA" = 1 ]; then
    printf '%s' "$data" | while IFS= read -r p; do
        [ -n "$p" ] && say "  remove $p (your data)"
    done
elif [ -n "$data" ]; then
    say "Kept in $BENCH_HOME: config.toml, index, parties, house-rules, encounters, reports."
fi

if [ "$DRY_RUN" = 1 ]; then
    say "Dry run. Nothing was changed."
    exit 0
fi

if [ -n "$targets" ] || [ -n "$profiles" ]; then
    confirm "Remove bench and its skill copies?" || { say "Stopped. Nothing was changed."; exit 0; }
fi

printf '%s' "$targets" | while IFS= read -r p; do
    if [ -n "$p" ]; then
        rm -rf "$p"
        say "removed $p"
    fi
done

printf '%s' "$profiles" | while IFS= read -r p; do
    if [ -n "$p" ]; then
        tmp=$(mktemp)
        grep -vFx "$path_line" "$p" >"$tmp" || true
        cat "$tmp" >"$p"
        rm -f "$tmp"
        say "removed the PATH line from $p"
    fi
done

if [ "$REMOVE_DATA" = 1 ] && [ -n "$data" ]; then
    if confirm "Delete your data in $BENCH_HOME too? This cannot be undone."; then
        printf '%s' "$data" | while IFS= read -r p; do
            if [ -n "$p" ]; then
                rm -rf "$p"
                say "removed $p"
            fi
        done
    else
        say "Kept your data."
    fi
fi

rmdir "$BENCH_HOME/bin" 2>/dev/null || true
rmdir "$BENCH_HOME/skills" 2>/dev/null || true
rmdir "$BENCH_HOME" 2>/dev/null || true

say ""
say "bench is removed. Open a new terminal so the PATH change applies."
if [ -d "$BENCH_HOME" ]; then
    say "Your data is still in $BENCH_HOME. Run with --remove-data to delete it."
fi
