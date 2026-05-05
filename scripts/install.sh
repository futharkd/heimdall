#!/bin/sh
# Install heimdall-linux-amd64 from GitHub Releases.
#
# Environment (optional):
#   HEIMDALL_VERSION   Package channel (default: latest). Example: v0.1.0
#   HEIMDALL_INSTALL_DIR  Directory for the binary (default: /usr/local/bin as root, else ~/.local/bin)
#   GITHUB_TOKEN  Optional token for authentication (redacted in output).
#
# Usage:
#   curl -fsSL "https://raw.githubusercontent.com/futharkd/heimdall/main/scripts/install.sh" | sh

set -eu

readonly github_repo='futharkd/heimdall'
readonly artifact='heimdall-linux-amd64'

die() {
	printf '%s\n' "$*" >&2
	exit 1
}

need_cmd() {
	command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

need_cmd curl

if [ "$(uname -s)" != 'Linux' ]; then
	die 'this installer only supports Linux'
fi

if [ "$(uname -m)" != 'x86_64' ]; then
	die 'this installer only supports x86_64 (amd64)'
fi

version=${HEIMDALL_VERSION:-latest}

if [ "$(id -u)" -eq 0 ]; then
	dest_dir=${HEIMDALL_INSTALL_DIR:-/usr/local/bin}
else
	dest_dir=${HEIMDALL_INSTALL_DIR:-"$HOME/.local/bin"}
fi

case $dest_dir in
/*) ;;
*)
	die "HEIMDALL_INSTALL_DIR must be an absolute path (got: $dest_dir)"
	;;
esac

if [ "$version" = "latest" ]; then
	bin_url="https://github.com/${github_repo}/releases/latest/download/${artifact}"
	sha_url="https://github.com/${github_repo}/releases/latest/download/${artifact}.sha256"
else
	bin_url="https://github.com/${github_repo}/releases/download/${version}/${artifact}"
	sha_url="https://github.com/${github_repo}/releases/download/${version}/${artifact}.sha256"
fi

tmpdir=
cleanup() {
	if [ -n "$tmpdir" ] && [ -d "$tmpdir" ]; then
		rm -rf "$tmpdir"
	fi
}
trap cleanup EXIT INT HUP TERM
tmpdir=$(mktemp -d)

tmp_sha="${tmpdir}/${artifact}.sha256"
tmp_bin="${tmpdir}/${artifact}"

curl_download() {
	url=$1
	out=$2
	if [ -n "${GITHUB_TOKEN:-}" ]; then
		curl -fSL -H "Authorization: token ${GITHUB_TOKEN}" -o "$out" "$url"
	else
		curl -fSL -o "$out" "$url"
	fi
}

printf 'Downloading %s ...\n' "$sha_url"
curl_download "$sha_url" "$tmp_sha"

printf 'Downloading %s ...\n' "$bin_url"
curl_download "$bin_url" "$tmp_bin"

expected=$(awk 'NF { print $1; exit }' "$tmp_sha") || die 'could not read digest from .sha256 file'
if [ "${#expected}" -ne 64 ]; then
	die "unexpected digest length in .sha256 (expected 64 hex chars)"
fi

if command -v sha256sum >/dev/null 2>&1; then
	actual=$(sha256sum "$tmp_bin" | awk '{ print $1 }')
elif command -v openssl >/dev/null 2>&1; then
	actual=$(openssl dgst -sha256 "$tmp_bin" | awk '{ print $NF }')
else
	die 'need sha256sum or openssl to verify the download'
fi

if [ "$actual" != "$expected" ]; then
	die "checksum mismatch (expected $expected, got $actual)"
fi

chmod +x "$tmp_bin"

mkdir -p "$dest_dir"
install_path="${dest_dir}/heimdall"
if ! mv -f "$tmp_bin" "$install_path" 2>/dev/null; then
	die "could not install to $install_path (try mkdir -p $dest_dir, fix permissions, or set HEIMDALL_INSTALL_DIR)"
fi

printf '\nInstalled heimdall to %s\n' "$install_path"
case ":${PATH:-}:" in
*":${dest_dir}:"*) ;;
*)
	printf 'Hint: add to your PATH for this shell session:\n  export PATH="%s:$PATH"\n' "$dest_dir"
	;;
esac

printf 'Verify with:\n  %s doctor\n' "$install_path"
