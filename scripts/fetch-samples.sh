#!/usr/bin/env bash
# fetch-samples.sh — download the real sample files that the tests read when
# they are present (CONTRIBUTING.md §Code style and checks). The list is
# tests/samples.tsv: one file per line, pinned by its SHA-256, each with the
# site it comes from, its id there, its tier, its licence and its URL.
# Without the files, those tests skip.
#
#   scripts/fetch-samples.sh                     # every file of the normal tier
#   scripts/fetch-samples.sh jpeg                # only those of one format:
#                                                # nef, arw or jpeg
#   RAW_FILES_LARGE=1 scripts/fetch-samples.sh   # the large tier as well
#
# A file of the "large" tier, such as the 325 MiB ARQ, is skipped with a
# note unless RAW_FILES_LARGE is 1, so a plain run stays at the size CI
# downloads. A format given as the argument limits either run to the files
# of that format; a run without RAW_FILES_LARGE whose every file is in the
# large tier fails, saying so, rather than pass having fetched nothing.
#
# The files go to $RAW_FILES_SAMPLES (relative to the current directory), or
# to raw-samples in the repository when it is unset, a folder that git
# ignores and that cargo clean leaves alone, unlike target; cargo test looks
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
dest=${dest:-raw-samples}

# The RAW files come from raw.pixls.us and the JPEG files from Wikimedia
# Commons, as the list's source column says, and each site gets its own
# User-Agent. raw.pixls.us gets a browser's: a site that filters bots may
# refuse curl's own. Commons asks a script for one that names it and a way
# to reach its authors, and to go easy on its servers, so the script waits
# a moment before each download from Commons after the first.
browser_agent='Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36'
commons_agent='raw-files-fetch-samples/1.0 (https://github.com/kuolas/raw-files)'
commons_pause=2

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
if [ "$header" != "$(printf 'source\tid\tformat\tfile\ttier\tsha256\tlicence\turl')" ]; then
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
skipped=0
from_commons=0
while IFS=$'\t' read -r source id format file tier sha256 licence url || [ -n "$source" ]; do
	[ -n "$source" ] || continue
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
	case "$tier" in
		normal | large) ;;
		*)
			printf 'fetch-samples: %s: bad tier "%s" for %s; it must be normal or large\n' "$list" "$tier" "$file" >&2
			exit 2
			;;
	esac
	if ! [[ "$sha256" =~ ^[0-9a-f]{64}$ ]] || [ -z "$url" ]; then
		printf 'fetch-samples: %s: line for %s needs a lower-case SHA-256 and a URL\n' "$list" "$file" >&2
		exit 2
	fi
	# The source picks the User-Agent, so the URL must be on its site.
	case "$source" in
		raw.pixls.us) site='https://raw.pixls.us/' agent=$browser_agent ;;
		commons) site='https://upload.wikimedia.org/' agent=$commons_agent ;;
		*)
			printf 'fetch-samples: %s: bad source "%s" for %s; it must be raw.pixls.us or commons\n' "$list" "$source" "$file" >&2
			exit 2
			;;
	esac
	if [ "${url#"$site"}" = "$url" ]; then
		printf 'fetch-samples: %s: the URL of %s is not on %s, its source\n' "$list" "$file" "$site" >&2
		exit 2
	fi

	# A large file is skipped only after its line has been checked, so a bad
	# line fails even when its file is not fetched.
	if [ "$tier" = large ] && [ "${RAW_FILES_LARGE:-}" != 1 ]; then
		printf 'skipped  %s (%s %s, large; RAW_FILES_LARGE=1 fetches it)\n' "$file" "$source" "$id"
		skipped=$((skipped + 1))
		continue
	fi

	target="$dest/$file"
	if [ -f "$target" ] && [ "$(sha256 "$target")" = "$sha256" ]; then
		printf 'kept     %s (%s %s, %s)\n' "$file" "$source" "$id" "$licence"
		continue
	fi

	if [ "$source" = commons ]; then
		if [ "$from_commons" -gt 0 ]; then sleep "$commons_pause"; fi
		from_commons=$((from_commons + 1))
	fi
	part=$(mktemp "$dest/.$file.XXXXXX")
	curl -fsSL --retry 3 --proto '=https' -A "$agent" -o "$part" "$url"
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
	printf 'fetched  %s (%s %s, %s)\n' "$file" "$source" "$id" "$licence"
done < <(tail -n +2 "$list")

if [ "$matched" -eq 0 ]; then
	printf 'fetch-samples: no samples%s in %s\n' "${only:+ of format $only}" "$list" >&2
	exit 2
fi
if [ "$skipped" -eq "$matched" ]; then
	printf 'fetch-samples: nothing fetched: the %d sample(s)%s are all in the large tier; RAW_FILES_LARGE=1 fetches them\n' "$matched" "${only:+ of format $only}" >&2
	exit 2
fi
