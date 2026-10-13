#!/usr/bin/env bash
# fetch-samples.sh — download the real sample files that the tests read when
# they are present (CONTRIBUTING.md §Code style and checks). The list is
# tests/samples.tsv: one file per line, pinned by its SHA-256, each with its
# licence and the URL it comes from. Without the files, those tests skip.
#
#   scripts/fetch-samples.sh             # every file in the list
#   scripts/fetch-samples.sh nef         # only the files of one format
#
# The files go to $RAW_FILES_SAMPLES (relative to the current directory), or
# to target/raw-samples in the repository when it is unset; cargo test looks
# in the same place. A file already there whose hash matches is kept, so a
# second run downloads nothing. A download whose hash differs is deleted, and
# the script fails naming the file and both hashes.
set -euo pipefail

usage() {
	printf 'usage: scripts/fetch-samples.sh [format]\n' >&2
	exit 2
}

[ $# -le 1 ] || usage
only=${1:-}
case "$only" in -*) usage ;; esac

# A relative RAW_FILES_SAMPLES is taken from where the script is run, before
# the cd below; the default is relative to the repository.
dest=${RAW_FILES_SAMPLES:-}
if [ -n "$dest" ] && [ "${dest#/}" = "$dest" ]; then
	dest="$PWD/$dest"
fi

cd "$(dirname "$0")/.."

list=tests/samples.tsv
dest=${dest:-target/raw-samples}

# A browser's User-Agent: a site that filters bots may refuse curl's own.
user_agent='Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36'

# Linux has sha256sum; macOS has shasum.
if command -v sha256sum >/dev/null 2>&1; then
	hasher=(sha256sum)
else
	hasher=(shasum -a 256)
fi

sha256() {
	local out
	out=$("${hasher[@]}" "$1")
	printf '%s\n' "${out%% *}"
}

header=$(head -n 1 "$list")
if [ "$header" != "$(printf 'id\tformat\tfile\tsha256\tlicence\turl')" ]; then
	printf 'fetch-samples: %s does not start with the expected header line\n' "$list" >&2
	exit 2
fi

mkdir -p "$dest"

# The download in progress, removed if the script stops before it is checked.
part=""
cleanup() {
	if [ -n "$part" ]; then rm -f "$part"; fi
}
trap cleanup EXIT

matched=0
while IFS=$'\t' read -r id format file sha256 licence url || [ -n "$id" ]; do
	[ -n "$id" ] || continue
	if [ -n "$only" ] && [ "$format" != "$only" ]; then
		continue
	fi
	matched=$((matched + 1))

	# The name becomes a path under $dest, so it must stay there.
	case "$file" in
		'' | .* | */*)
			printf 'fetch-samples: %s: bad file name "%s" for id %s\n' "$list" "$file" "$id" >&2
			exit 2
			;;
	esac
	if ! [[ "$sha256" =~ ^[0-9a-f]{64}$ ]] || [ -z "$url" ]; then
		printf 'fetch-samples: %s: line for %s needs a lower-case SHA-256 and a URL\n' "$list" "$file" >&2
		exit 2
	fi

	target="$dest/$file"
	if [ -f "$target" ] && [ "$(sha256 "$target")" = "$sha256" ]; then
		printf 'kept     %s (%s, %s)\n' "$file" "$id" "$licence"
		continue
	fi

	part=$(mktemp "$dest/.$file.XXXXXX")
	curl -fsSL --retry 3 --proto '=https' -A "$user_agent" -o "$part" "$url"
	got=$(sha256 "$part")
	if [ "$got" != "$sha256" ]; then
		rm -f "$part"
		part=""
		printf 'fetch-samples: %s: SHA-256 mismatch, download deleted\n' "$file" >&2
		printf '  expected %s\n' "$sha256" >&2
		printf '  got      %s\n' "$got" >&2
		exit 1
	fi
	# mktemp made it readable by its owner only.
	chmod 644 "$part"
	mv -f "$part" "$target"
	part=""
	printf 'fetched  %s (%s, %s)\n' "$file" "$id" "$licence"
done < <(tail -n +2 "$list")

if [ "$matched" -eq 0 ]; then
	printf 'fetch-samples: no samples%s in %s\n' "${only:+ of format $only}" "$list" >&2
	exit 2
fi
