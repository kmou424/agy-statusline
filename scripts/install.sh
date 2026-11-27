#!/bin/sh
set -eu

repo=https://github.com/kmou424/agy-statusline

usage() {
    printf '%s\n' 'Usage: install.sh [--canary | --version TAG] [--install-dir DIR]' \
        '                  [--no-configure] [--no-title] [--stack-with-default]' \
        'Installs the latest stable release by default.'
}

die() {
    printf 'agy-statusline: %s\n' "$*" >&2
    exit 1
}

valid_tag() {
    case "$1" in
        *[!abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._+-]*) return 1 ;;
        v[0123456789]*) return 0 ;;
        *) return 1 ;;
    esac
}

supports_title_tag() {
    [ "$1" != canary ] || return 0
    version_core=${1%%[-+]*}
    case "$version_core" in
        v*.*) version_core=${version_core#v} ;;
        *) return 1 ;;
    esac
    major=${version_core%%.*}
    minor_patch=${version_core#*.}
    minor=${minor_patch%%.*}
    patch=0
    case "$minor_patch" in
        *.*) patch=${minor_patch#*.} ;;
    esac
    for component in "$major" "$minor" "$patch"; do
        case "$component" in
            ''|*[!0123456789]*) return 1 ;;
        esac
    done

    major=${major#"${major%%[!0]*}"}
    minor=${minor#"${minor%%[!0]*}"}
    patch=${patch#"${patch%%[!0]*}"}
    [ -n "$major" ] || { [ -n "$minor" ] && [ "$minor" != 1 ]; } ||
        { [ "$minor" = 1 ] && [ -n "$patch" ] && [ "$patch" != 1 ]; }
}

download() {
    case "$1" in
        file:///*)
            if [ -f "${1#file://}" ]; then
                cp "${1#file://}" "$2"
                return
            fi
            ;;
    esac
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$2" "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$2" "$1"
    else
        die 'Install curl or wget to download release assets.'
    fi
}

latest_tag() {
    if [ -n "${AGY_STATUSLINE_LATEST_TAG:-}" ]; then
        printf '%s\n' "$AGY_STATUSLINE_LATEST_TAG"
        return
    fi
    if command -v curl >/dev/null 2>&1 &&
        latest_url=$(curl -fsSL -o /dev/null -w '%{url_effective}' "$repo/releases/latest"); then
        case "$latest_url" in
            "$repo/releases/tag/"*)
                latest=${latest_url#"$repo/releases/tag/"}
                if valid_tag "$latest"; then
                    printf '%s\n' "$latest"
                    return
                fi
                ;;
        esac
    fi
    if download https://api.github.com/repos/kmou424/agy-statusline/releases/latest "$tmp_dir/latest.json"; then
        awk 'match($0, /"tag_name"[[:space:]]*:[[:space:]]*"[^"]+"/) {
            tag = substr($0, RSTART, RLENGTH)
            sub(/^[^:]+:[[:space:]]*"/, "", tag)
            sub(/"$/, "", tag)
            print tag
            exit
        }' "$tmp_dir/latest.json"
    fi
}

detect_target() {
    os=${AGY_STATUSLINE_OS:-$(uname -s)}
    arch=${AGY_STATUSLINE_ARCH:-$(uname -m)}
    if [ -z "${AGY_STATUSLINE_OS:-}" ]; then
        case "$(uname -o 2>/dev/null || :)" in
            *Android*) os=Android ;;
        esac
        if [ -n "${TERMUX_VERSION:-}" ] || [ -x /system/bin/app_process ] ||
            [ -x /system/bin/app_process64 ]; then
            os=Android
        fi
    fi
    case "$arch" in
        arm64|aarch64) cpu=aarch64 ;;
        amd64|x86_64) cpu=x86_64 ;;
        *) die "Unsupported architecture: $arch" ;;
    esac
    case "$os" in
        Android|android|Termux|termux) printf '%s-linux-android\n' "$cpu" ;;
        Linux|linux) printf '%s-unknown-linux-musl\n' "$cpu" ;;
        Darwin|darwin)
            if [ "$arch" = x86_64 ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || :)" = 1 ]; then
                cpu=aarch64
            fi
            printf '%s-apple-darwin\n' "$cpu"
            ;;
        *) die "Unsupported operating system: $os" ;;
    esac
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        hash_output=$(sha256sum < "$1") || return 1
        printf '%s\n' "${hash_output%% *}"
    elif command -v shasum >/dev/null 2>&1; then
        hash_output=$(shasum -a 256 < "$1") || return 1
        printf '%s\n' "${hash_output%% *}"
    elif command -v openssl >/dev/null 2>&1; then
        hash_output=$(openssl dgst -sha256 < "$1") || return 1
        printf '%s\n' "${hash_output##* }"
    else
        die 'Install sha256sum, shasum, or openssl to verify the download.'
    fi
}

shell_quote() {
    case "$1" in
        ''|*[!abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_./:~-]*)
            printf "'"
            printf '%s' "$1" | sed "s/'/'\\\\''/g"
            printf "'"
            ;;
        *)
            printf '%s' "$1"
            ;;
    esac
}

resolve_target_path() {
    if [ -L "$1" ]; then
        if command -v python3 >/dev/null 2>&1; then
            python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$1"
            return
        elif command -v readlink >/dev/null 2>&1 && resolved=$(readlink -f "$1" 2>/dev/null); then
            printf '%s\n' "$resolved"
            return
        fi
    fi
    printf '%s\n' "$1"
}

configure() {
    settings_dir=$HOME/.gemini/antigravity-cli
    settings=$settings_dir/settings.json
    if [ "$install_dir" = "$default_dir" ]; then
        # agy expands the tilde on each machine.
        # shellcheck disable=SC2088
        status_command='~/.local/bin/agy-statusline'
    else
        status_command=$(shell_quote "$bin_path")
    fi
    title_command="$status_command title"
    source_json=/dev/null
    target_settings=$settings
    if [ -e "$settings" ] || [ -L "$settings" ]; then
        [ -f "$settings" ] || die "Expected a settings file at $settings."
        target_settings=$(resolve_target_path "$settings")
        source_json=$target_settings
    fi
    if command -v python3 >/dev/null 2>&1; then
        editor=python3
    elif command -v jq >/dev/null 2>&1; then
        editor=jq
    elif command -v node >/dev/null 2>&1; then
        editor=node
    elif [ "$source_json" != /dev/null ]; then
        printf 'agy-statusline: Install python3, jq, or node to merge settings, or run in agy:\n/statusline %s\n' \
            "$status_command" >&2
        if [ "$configure_title" = true ]; then
            printf 'Set the "title" command in ~/.gemini/antigravity-cli/settings.json to: %s\n' \
                "$title_command" >&2
        fi
        return
    else
        editor='awk'
    fi

    target_dir=$(dirname "$target_settings")
    mkdir -p "$target_dir"
    settings_tmp=$(mktemp "$target_dir/.settings.json.XXXXXXXX")
    case "$editor" in
        python3)
            python3 - "$source_json" "$status_command" "$stack" "$title_command" "$configure_title" > "$settings_tmp" <<'PY'
import json
import sys

source, command, stack, title_command, configure_title = sys.argv[1:]
with open(source, encoding="utf-8-sig") as f:
    settings = {} if source == "/dev/null" else json.load(f)
if not isinstance(settings, dict):
    sys.exit("settings.json must contain a JSON object")
line = settings.get("statusLine")
if not isinstance(line, dict):
    line = {}
line.update(type="command", command=command, enabled=True)
if stack == "true":
    line["stack_with_default"] = True
settings["statusLine"] = line
if configure_title == "true":
    title = settings.get("title")
    if not isinstance(title, dict):
        title = {}
    title.update(type="command", command=title_command, enabled=True)
    settings["title"] = title
print(json.dumps(settings, indent=2))
PY
            ;;
        jq)
            jq -es --arg command "$status_command" --argjson stack "$stack" \
                --arg title_command "$title_command" --argjson configure_title "$configure_title" \
                --arg source "$source_json" '
                if $source == "/dev/null" then {}
                elif length == 1 and (.[0] | type == "object") then .[0]
                else error("settings.json must contain a JSON object") end
                | .statusLine |= (
                    if type == "object" then . else {} end
                    | . + {type: "command", command: $command, enabled: true}
                    | if $stack then .stack_with_default = true else . end
                )
                | if $configure_title then
                    .title |= (
                        if type == "object" then . else {} end
                        | . + {type: "command", command: $title_command, enabled: true}
                    )
                  else . end' "$source_json" > "$settings_tmp"
            ;;
        node)
            node - "$source_json" "$status_command" "$stack" "$title_command" "$configure_title" > "$settings_tmp" <<'JS'
const fs = require("fs");
const [source, command, stack, titleCommand, configureTitle] = process.argv.slice(2);
const settings = source === "/dev/null" ? {} :
    JSON.parse(fs.readFileSync(source, "utf8").replace(/^\uFEFF/, ""));
const isObject = value => value !== null && typeof value === "object" && !Array.isArray(value);
if (!isObject(settings)) throw new Error("settings.json must contain a JSON object");
const existing = isObject(settings.statusLine) ? settings.statusLine : {};
const line = { ...existing, type: "command", command, enabled: true };
if (stack === "true") line.stack_with_default = true;
settings.statusLine = line;
if (configureTitle === "true") {
    const title = isObject(settings.title) ? settings.title : {};
    settings.title = { ...title, type: "command", command: titleCommand, enabled: true };
}
console.log(JSON.stringify(settings, null, 2));
JS
            ;;
        awk)
            AGY_INSTALL_COMMAND=$status_command AGY_INSTALL_TITLE_COMMAND=$title_command LC_ALL=C \
                awk -v stack="$stack" -v configure_title="$configure_title" '
                function json_escape(value,    i, c, escaped) {
                    escaped = ""
                    for (i = 1; i <= length(value); i++) {
                        c = substr(value, i, 1)
                        if (c in control) escaped = escaped control[c]
                        else if (c == "\"" || c == "\\") escaped = escaped "\\" c
                        else escaped = escaped c
                    }
                    return escaped
                }
                BEGIN {
                    for (i = 1; i < 32; i++) {
                        control[sprintf("%c", i)] = sprintf("\\u%04x", i)
                    }
                    printf "{\"statusLine\":{\"type\":\"command\",\"command\":\"%s\",\"enabled\":true",
                        json_escape(ENVIRON["AGY_INSTALL_COMMAND"])
                    if (stack == "true") printf ",\"stack_with_default\":true"
                    printf "}"
                    if (configure_title == "true") {
                        printf ",\"title\":{\"type\":\"command\",\"command\":\"%s\",\"enabled\":true}",
                            json_escape(ENVIRON["AGY_INSTALL_TITLE_COMMAND"])
                    }
                    print "}"
                }' > "$settings_tmp"
            ;;
    esac
    mv -f "$settings_tmp" "$target_settings"
    settings_tmp=
}

cleanup() {
    if [ -n "$bin_tmp" ]; then rm -f "$bin_tmp"; fi
    if [ -n "$settings_tmp" ]; then rm -f "$settings_tmp"; fi
    rm -rf "$tmp_dir"
}

main() {
    canary=false
    version=
    custom_dir=
    no_configure=false
    configure_title=true
    stack=false
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --canary) canary=true ;;
            --no-configure) no_configure=true ;;
            --no-title) configure_title=false ;;
            --stack-with-default) stack=true ;;
            --version|--version=*)
                case "$1" in
                    --version) [ "$#" -gt 1 ] || die '--version needs a tag.'; shift; version=$1 ;;
                    *) version=${1#*=} ;;
                esac
                valid_tag "$version" || die "Invalid release tag: $version"
                ;;
            --install-dir|--install-dir=*)
                case "$1" in
                    --install-dir) [ "$#" -gt 1 ] || die '--install-dir needs a directory.'; shift; custom_dir=$1 ;;
                    *) custom_dir=${1#*=} ;;
                esac
                [ -n "$custom_dir" ] || die '--install-dir needs a directory.'
                ;;
            -h|--help) usage; exit 0 ;;
            *) die "Unknown option: $1 (see --help)" ;;
        esac
        shift
    done
    if [ "$canary" = true ] && [ -n "$version" ]; then
        die 'Choose either --canary or --version.'
    fi

    default_dir=$HOME/.local/bin
    install_dir=${custom_dir:-$default_dir}
    case "$install_dir" in
        /*) ;;
        *) install_dir=$PWD/$install_dir ;;
    esac
    bin_path=${install_dir%/}/agy-statusline
    target=$(detect_target)
    bin_tmp=
    settings_tmp=
    tmp_dir=$(mktemp -d)
    trap cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM

    if [ "$canary" = true ]; then
        tag=canary
    elif [ -n "$version" ]; then
        tag=$version
    else
        if ! tag=$(latest_tag) || ! valid_tag "$tag"; then
            die 'Could not find a stable release. Try --canary.'
        fi
    fi
    archive=agy-statusline-$tag-$target.tar.gz
    base=${AGY_STATUSLINE_RELEASE_BASE_URL:-$repo/releases/download}
    asset_dir=${base%/}/$tag
    download "$asset_dir/$archive" "$tmp_dir/$archive" || die "Could not download $archive."
    download "$asset_dir/SHA256SUMS" "$tmp_dir/SHA256SUMS" || die 'Could not download SHA256SUMS.'
    expected=$(awk -v name="$archive" '
        substr($0, 65) == "  " name {
            sum = substr($0, 1, 64)
            if (sum ~ /^[[:xdigit:]]+$/) { count++; hash = tolower(sum) }
        }
        END { if (count == 1) print hash; else exit 1 }
    ' "$tmp_dir/SHA256SUMS") || die "Missing or invalid SHA256SUMS entry for $archive."
    actual=$(sha256 "$tmp_dir/$archive") || die "Could not hash $archive."
    [ "$actual" = "$expected" ] || die "SHA-256 mismatch for $archive."

    tar -xzf "$tmp_dir/$archive" -C "$tmp_dir" agy-statusline
    [ -f "$tmp_dir/agy-statusline" ] && [ ! -L "$tmp_dir/agy-statusline" ] ||
        die 'Archive must contain a regular agy-statusline binary.'
    mkdir -p "$install_dir"
    [ ! -d "$bin_path" ] || die "Expected a binary, found a directory at $bin_path."
    bin_tmp=$(mktemp "$install_dir/.agy-statusline.XXXXXXXX")
    cp "$tmp_dir/agy-statusline" "$bin_tmp"
    chmod 755 "$bin_tmp"
    mv -f "$bin_tmp" "$bin_path"
    bin_tmp=

    if [ "$no_configure" = false ]; then
        if [ "$configure_title" = true ] && ! supports_title_tag "$tag"; then
            configure_title=false
            printf 'agy-statusline: Skipping title configuration (%s does not include the title subcommand).\n' "$tag" >&2
        fi
        configure
    fi
    printf 'Installed %s to %s\n' "$tag" "$bin_path"
}

main "$@"
