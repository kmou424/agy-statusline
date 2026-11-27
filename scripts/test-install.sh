#!/bin/sh
set -eu

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
installer=$script_dir/install.sh
installer_shell=sh
if [ -n "${BASH_VERSION:-}" ]; then installer_shell=bash; fi
installer_shell=$(command -v "$installer_shell")
tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

fail() {
    printf 'test-install: %s\n' "$*" >&2
    exit 1
}

release_root=$tmp_dir/releases
export HOME=$tmp_dir/home
export AGY_STATUSLINE_RELEASE_BASE_URL=file://$release_root
export AGY_STATUSLINE_LATEST_TAG=v0.1.2
export AGY_STATUSLINE_OS=linux
export AGY_STATUSLINE_ARCH=x86_64
settings=$HOME/.gemini/antigravity-cli/settings.json
default_binary=$HOME/.local/bin/agy-statusline
mkdir -p "$HOME/.gemini/antigravity-cli" "$tmp_dir/payload" "$tmp_dir/dotfiles"
settings_real=$tmp_dir/dotfiles/settings.json
printf '%s\n' '{"other":{"keep":[1,true,"yes"]},"statusLine":{"padding":2,"enabled":false,"stack_with_default":true},"title":{"prefix":"keep","enabled":false}}' > "$settings_real"
ln -s "$settings_real" "$settings"
printf '%s\n' 'Mock release README' > "$tmp_dir/payload/README.md"

release() {
    tag=$1
    target=$2
    payload=$3
    archive=agy-statusline-$tag-$target.tar.gz
    dest=$release_root/$tag
    mkdir -p "$dest"
    printf '#!/bin/sh\npayload=%s\n' "$payload" > "$tmp_dir/payload/agy-statusline"
    printf '%s\n' 'if [ "$#" -gt 0 ]; then payload="$payload $*"; fi' \
        'printf "%s\n" "$payload"' >> "$tmp_dir/payload/agy-statusline"
    tar -czf "$dest/$archive" -C "$tmp_dir/payload" agy-statusline README.md
    hash=$(python3 -c 'import hashlib, sys; print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())' "$dest/$archive")
    printf '%s  %s\n' "$hash" "$archive" >> "$dest/SHA256SUMS"
}

run_installer() {
    "$installer_shell" -s -- "$@" < "$installer"
}

check_settings() {
    python3 - "$settings" "$1" "$2" "${3:-true}" <<'PY'
import json
import sys

source, command, title_command, title_enabled = sys.argv[1:]
with open(source) as f:
    data = json.load(f)
assert data["other"] == {"keep": [1, True, "yes"]}, data
assert data["statusLine"] == {
    "type": "command", "command": command, "enabled": True, "padding": 2,
    "stack_with_default": True,
}, data
assert data["title"] == {
    "type": "command", "command": title_command, "enabled": title_enabled == "true", "prefix": "keep",
}, data
PY
}

release v0.1.2 x86_64-unknown-linux-musl stable
release canary x86_64-unknown-linux-musl canary
release v0.1.1 x86_64-unknown-linux-musl legacy
release v0.1.2 aarch64-linux-android android

run_installer
[ -x "$default_binary" ] && [ "$("$default_binary")" = stable ] || fail 'stable binary'
[ -L "$settings" ] || fail 'installer replaced settings symlink'
check_settings '~/.local/bin/agy-statusline' '~/.local/bin/agy-statusline title'

# Keep the original inode open through replacement.
exec 3< "$default_binary"
run_installer --canary
[ "$("$default_binary")" = canary ] || fail 'canary binary'
[ "$(sh <&3)" = stable ] || fail 'canary must leave the old open inode intact'
exec 3<&-

legacy_dir="$tmp_dir/legacy dir"
run_installer --version v0.1.1 --install-dir "$legacy_dir" 2> "$tmp_dir/legacy.stderr"
[ "$("$legacy_dir/agy-statusline")" = legacy ] || fail 'legacy binary'
check_settings "'$legacy_dir/agy-statusline'" '~/.local/bin/agy-statusline title'
grep -Fx 'agy-statusline: Skipping title configuration (v0.1.1 does not include the title subcommand).' \
    "$tmp_dir/legacy.stderr" >/dev/null || fail 'legacy title skip diagnostic'

custom_dir="$tmp_dir/custom dir"
run_installer --version=v0.1.2 --install-dir="$custom_dir" --stack-with-default
[ "$("$custom_dir/agy-statusline")" = stable ] || fail 'custom binary'
check_settings "'$custom_dir/agy-statusline'" "'$custom_dir/agy-statusline' title"
saved_cmd=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["statusLine"]["command"])' "$settings")
[ "$(sh -c "$saved_cmd")" = stable ] || fail 'saved custom command failed under sh -c'
saved_title_cmd=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["title"]["command"])' "$settings")
[ "$(sh -c "$saved_title_cmd")" = 'stable title' ] || fail 'saved title command failed under sh -c'

python3 - "$settings_real" <<'PY'
import json
import sys

with open(sys.argv[1], "r+") as f:
    data = json.load(f)
    data["title"]["command"] = "custom-title"
    data["title"]["enabled"] = False
    f.seek(0)
    json.dump(data, f)
    f.truncate()
PY
run_installer --no-title
check_settings '~/.local/bin/agy-statusline' 'custom-title' false
cp "$settings" "$tmp_dir/settings.before"
run_installer --version v0.1.2 --install-dir "$custom_dir" --no-configure
cmp "$tmp_dir/settings.before" "$settings" || fail '--no-configure'

(
    export AGY_STATUSLINE_OS=android AGY_STATUSLINE_ARCH=aarch64
    run_installer --no-configure
)
[ "$("$default_binary")" = android ] || fail 'Android binary'

manifest=$release_root/canary/SHA256SUMS
printf '%064d  %s\n' 0 agy-statusline-canary-x86_64-unknown-linux-musl.tar.gz > "$manifest"
if run_installer --canary > "$tmp_dir/checksum.log" 2>&1; then
    fail 'checksum mismatch accepted'
fi
grep -F 'SHA-256 mismatch' "$tmp_dir/checksum.log" >/dev/null || fail 'checksum mismatch diagnostic'
[ "$("$default_binary")" = android ] || fail 'checksum mismatch replaced binary'
cmp "$tmp_dir/settings.before" "$settings" || fail 'checksum mismatch changed settings'

printf '%s\n' 'No matching archive entry' > "$manifest"
if run_installer --canary > "$tmp_dir/checksum.log" 2>&1; then
    fail 'missing checksum accepted'
fi
grep -F 'Missing or invalid SHA256SUMS entry' "$tmp_dir/checksum.log" >/dev/null || fail 'missing checksum diagnostic'
[ "$("$default_binary")" = android ] || fail 'missing checksum replaced binary'

# Exercise the fallback with only the installer utilities on PATH.
tool_path=$tmp_dir/tools
mkdir "$tool_path"
for tool in awk chmod cp dirname gzip mkdir mktemp mv rm sed tar; do
    ln -s "$(command -v "$tool")" "$tool_path/$tool"
done
for tool in sha256sum shasum openssl; do
    if tool_source=$(command -v "$tool"); then
        ln -s "$tool_source" "$tool_path/$tool"
        break
    fi
done
(
    export HOME=$tmp_dir/minimal
    minimal_dir=$HOME'/bin"\custom'$(printf '\t\ncontrol')
    minimal_settings=$HOME/.gemini/antigravity-cli/settings.json
    PATH=$tool_path "$installer_shell" -s -- --install-dir "$minimal_dir" --stack-with-default < "$installer"
    python3 - "$minimal_settings" "'$minimal_dir/agy-statusline'" <<'PY'
import json
import sys

with open(sys.argv[1]) as f:
    assert json.load(f) == {
        "statusLine": {
            "type": "command", "command": sys.argv[2], "enabled": True, "stack_with_default": True,
        },
        "title": {"type": "command", "command": sys.argv[2] + " title", "enabled": True},
    }
PY
    cp "$minimal_settings" "$tmp_dir/minimal.before"
    PATH=$tool_path "$installer_shell" -s -- < "$installer" > "$tmp_dir/minimal.log" 2>&1
    grep -Fx '/statusline ~/.local/bin/agy-statusline' "$tmp_dir/minimal.log" >/dev/null ||
        fail 'manual configuration command'
    grep -Fx 'Set the "title" command in ~/.gemini/antigravity-cli/settings.json to: ~/.local/bin/agy-statusline title' \
        "$tmp_dir/minimal.log" >/dev/null || fail 'manual title configuration command'
    cmp "$tmp_dir/minimal.before" "$minimal_settings" || fail 'fallback changed existing settings'

    PATH=$tool_path "$installer_shell" -s -- --no-title < "$installer" > "$tmp_dir/minimal.log" 2>&1
    grep -Fx '/statusline ~/.local/bin/agy-statusline' "$tmp_dir/minimal.log" >/dev/null ||
        fail 'manual configuration command with --no-title'
    if grep -F '"title"' "$tmp_dir/minimal.log" >/dev/null; then
        fail 'fallback printed title configuration with --no-title'
    fi
    cmp "$tmp_dir/minimal.before" "$minimal_settings" || fail 'fallback changed existing settings with --no-title'

    export HOME=$tmp_dir/minimal-no-title
    PATH=$tool_path "$installer_shell" -s -- --no-title < "$installer"
    python3 - "$HOME/.gemini/antigravity-cli/settings.json" <<'PY'
import json
import sys

with open(sys.argv[1]) as f:
    assert json.load(f) == {"statusLine": {
        "type": "command", "command": "~/.local/bin/agy-statusline", "enabled": True,
    }}
PY
)

printf '%s\n' 'Installer fixture tests passed.'
