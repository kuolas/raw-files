#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# test-hooks.sh — test .githooks/commit-msg: the grammar of CONTRIBUTING.md
# §Commit messages, on message files and through a real `git commit`. CI runs
# it; run it yourself after changing the hook.
#
#   scripts/test-hooks.sh
#
# Each case writes a message with printf '%b' (so `\n` is a newline) and
# checks whether the hook accepts or refuses it. Everything happens in a
# throwaway directory; your checkout and git config are untouched.
set -uo pipefail

SRC="$(cd "$(dirname "$0")/.." && pwd -P)"
HOOK="$SRC/.githooks/commit-msg"
W="$(mktemp -d "${TMPDIR:-/tmp}/raw-files-hooks.XXXXXX")"
trap 'rm -rf "$W"' EXIT
# No git config of yours applies (a signing key, a template): git reads
# $HOME/.gitconfig and $XDG_CONFIG_HOME/git/config, both moved here.
export HOME="$W/home" XDG_CONFIG_HOME="$W/home/.config" GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@example.invalid
export GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@example.invalid
unset SKIP_HOOKS COMMIT_MSG_STORED GIT_EDITOR
mkdir -p "$HOME"

FAILS=0
check() {
	local what="$1"
	shift
	if "$@"; then
		echo "PASS  $what"
	else
		echo "FAIL  $what"
		FAILS=$((FAILS + 1))
	fi
}

M="$W/msg"
accepts() { printf '%b' "$1" > "$M"; "$HOOK" "$M" > /dev/null 2>&1; }
refuses() { printf '%b' "$1" > "$M"; ! "$HOOK" "$M" > /dev/null 2>&1; }
# A message as git stored it, as CI reads it with `git log --format=%B`.
stored_refuses() { printf '%b' "$1" > "$M"; ! COMMIT_MSG_STORED=1 "$HOOK" "$M" > /dev/null 2>&1; }
# Accepted, with or without the note on a trailer outside the house set.
notes_trailer() {
	local err
	printf '%b' "$1" > "$M"
	err="$("$HOOK" "$M" 2>&1 > /dev/null)" && grep -q 'unsanctioned trailer' <<< "$err"
}
no_trailer_note() {
	local err
	printf '%b' "$1" > "$M"
	err="$("$HOOK" "$M" 2>&1 > /dev/null)" && ! grep -q 'unsanctioned trailer' <<< "$err"
}

echo '# The grammar'
check "accepts 'component: Summary' with Closes: and Assisted-by:" \
	accepts 'nef: Find the full-size preview in the SubIFDs\n\nA body line.\n\nCloses: #12\nAssisted-by: Claude\n'
check "accepts a subject without a tag" accepts 'Add a README\n'
# shellcheck disable=SC2016 # the backticks are the message, not a command
check "accepts a summary starting with an identifier in backticks" \
	accepts 'tiff: `read_ifd()` rejects offsets past the end\n'
check "accepts a person as co-author" \
	accepts 'nef: Fix it\n\nCo-authored-by: Jane Doe <jane@example.org>\n'
check "refuses a Conventional Commits subject" refuses 'feat(nef): Find the preview\n'
check "refuses two tags" refuses 'nef: tiff: Find the preview\n'
check "refuses a lower-case summary" refuses 'nef: find the preview\n'
check "refuses a trailing period" refuses 'nef: Find the preview.\n'
check "refuses a non-blank second line" refuses 'nef: Find the preview\nmore\n'
check "refuses Closes: without an issue reference" refuses 'nef: Fix it\n\nCloses: two\n'
check "refuses a tool as co-author" \
	refuses 'nef: Fix it\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n'
check "refuses a Generated-with footer" \
	refuses 'nef: Fix it\n\nBody.\n\nGenerated with [Claude Code](https://claude.com/claude-code)\n'
printf 'nef: find it.\n' > "$M"
skipped() { SKIP_HOOKS=1 "$HOOK" "$M" > /dev/null; }
check "lets SKIP_HOOKS through" skipped

echo '# Authorship: a tool is refused, a person or prose is not'
check "refuses Claude with Anthropic's address" \
	refuses 'nef: Fix it\n\nCo-authored-by: Claude <noreply@anthropic.com>\n'
check "refuses a named Claude model" \
	refuses 'nef: Fix it\n\nCo-authored-by: Claude Opus 4 <someone@example.invalid>\n'
check "refuses Devin AI" \
	refuses 'nef: Fix it\n\nCo-authored-by: Devin AI <158243242+devin-ai-integration[bot]@users.noreply.github.com>\n'
check "refuses Copilot's bot" \
	refuses 'nef: Fix it\n\nCo-authored-by: copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>\n'
check "refuses ChatGPT" refuses 'nef: Fix it\n\nCo-authored-by: ChatGPT <someone@example.invalid>\n'
check "refuses a GPT model" refuses 'nef: Fix it\n\nco-authored-by: GPT-4o <someone@example.invalid>\n'
check "refuses Cursor's agent" \
	refuses 'nef: Fix it\n\nCo-authored-by: Cursor Agent <cursoragent@cursor.com>\n'
check "refuses Gemini's bot" \
	refuses 'nef: Fix it\n\nCo-authored-by: gemini-code-assist[bot] <176961590+gemini-code-assist[bot]@users.noreply.github.com>\n'
check "refuses 'Generated with GitHub Copilot'" \
	refuses 'nef: Fix it\n\nBody.\n\nGenerated with GitHub Copilot\n'
check "refuses 'Generated with ChatGPT'" refuses 'nef: Fix it\n\nBody.\n\nGenerated with ChatGPT\n'
check "refuses claude[bot], as GitHub writes it" \
	refuses 'nef: Fix it\n\nCo-authored-by: claude[bot] <209825114+claude[bot]@users.noreply.github.com>\n'
check "refuses any [bot] login" \
	refuses 'nef: Fix it\n\nCo-authored-by: google-labs-jules[bot] <161369871+google-labs-jules[bot]@users.noreply.github.com>\n'
check "refuses Devin AI by its name alone" \
	refuses 'nef: Fix it\n\nCo-authored-by: Devin AI <someone@example.invalid>\n'
check "refuses Cursor by its name alone" \
	refuses 'nef: Fix it\n\nCo-authored-by: Cursor Agent <someone@example.invalid>\n'
check "refuses aider" \
	refuses 'nef: Fix it\n\nCo-authored-by: aider (some-model) <aider@aider.chat>\n'
check "refuses opencode by its name" \
	refuses 'nef: Fix it\n\nCo-authored-by: opencode (some-model) <ai@local>\n'
check "refuses Amp by its address" refuses 'nef: Fix it\n\nCo-authored-by: Amp <amp@ampcode.com>\n'
check "refuses Codebuff" refuses 'nef: Fix it\n\nCo-authored-by: Codebuff <noreply@codebuff.com>\n'
check "refuses 'Generated with Google Gemini'" \
	refuses 'nef: Fix it\n\nBody.\n\nGenerated with Google Gemini\n'
check "refuses 'Generated with the help of Claude Code'" \
	refuses 'nef: Fix it\n\nBody.\n\nGenerated with the help of Claude Code\n'
check "refuses 'Generated with assistance from ChatGPT'" \
	refuses 'nef: Fix it\n\nBody.\n\nGenerated with assistance from ChatGPT\n'
check "refuses 'Generated with Cursor'" refuses 'nef: Fix it\n\nBody.\n\nGenerated with Cursor\n'
check "accepts a person at a tool maker's domain" \
	accepts 'nef: Fix it\n\nCo-authored-by: Jane Doe <jane@anthropic.com>\nCo-authored-by: John Doe <john.doe@openai.com>\n'
check "accepts a person whose handle holds a tool's name" \
	accepts 'nef: Fix it\n\nCo-authored-by: Jane Doe <12345+cursor-jane@users.noreply.github.com>\n'
check "accepts wrapped prose about cursor reads" \
	accepts 'nef: Read offsets\n\nThe offsets in the IFD chain are all\ngenerated with cursor reads, never seeks.\n'
check "accepts a person named Claude" \
	accepts 'nef: Fix it\n\nCo-authored-by: Claude Monet <claude@example.org>\n'
check "accepts a domain holding a tool's letters" \
	accepts 'nef: Fix it\n\nCo-authored-by: Jane Doe <jane@egptech.org>\n'
check "accepts prose saying 'generated with a cursor'" \
	accepts 'nef: Read the offsets in order\n\nOffsets are generated with a cursor over the file.\n'
check "  and the same at the start of a line" \
	accepts 'nef: Read the offsets in order\n\nGenerated with a cursor over the file, the offsets\nare read in order.\n'
check "accepts prose naming the rule" \
	accepts 'hooks: Explain the rule\n\nThe hook refuses a Co-authored-by: line naming Claude Code.\n'

echo '# Trailer keys in any casing, as GitHub reads them'
check "refuses 'closes: two'" refuses 'nef: Fix it\n\nBody.\n\ncloses: two\n'
check "refuses 'FIXES: two'" refuses 'nef: Fix it\n\nBody.\n\nFIXES: two\n'
check "refuses 'issue: two'" refuses 'nef: Fix it\n\nBody.\n\nissue: two\n'
check "accepts 'fixes: #12'" accepts 'nef: Fix it\n\nBody.\n\nfixes: #12\n'
check "notes 'Reviewed-by:'" notes_trailer 'nef: Fix it\n\nBody.\n\nReviewed-by: Someone\n'
check "notes 'reviewed-by:'" notes_trailer 'nef: Fix it\n\nBody.\n\nreviewed-by: Someone\n'
check "does not note 'assisted-by:'" no_trailer_note 'nef: Fix it\n\nBody.\n\nassisted-by: Claude\n'
check "does not note 'Assisted-by:'" no_trailer_note 'nef: Fix it\n\nBody.\n\nAssisted-by: Claude\n'

echo '# Git'"'"'s own subjects: exempt from the grammar, not from the attribution rule'
check "accepts a revert as git writes it" accepts 'Revert "nef: Fix it"\n\nThis reverts commit 0123abc.\n'
check "accepts a reverted revert" accepts 'Reapply "nef: Fix it"\n\nThis reverts commit 0123abc.\n'
check "accepts a merge of a branch" accepts "Merge branch 'wip/topic'\n"
check "accepts GitHub's merge subject" accepts 'Merge pull request #12 from someone/wip/topic\n'
check "accepts fixup!" \
	accepts 'fixup! nef: Fix it\n'
check "  squash!" accepts 'squash! nef: Fix it\n'
check "  amend!" accepts 'amend! nef: Fix it\n\nnef: Fix it properly\n'
check "refuses a revert naming a tool as co-author" \
	refuses 'Revert "nef: Fix it"\n\nThis reverts commit 0123abc.\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n'
check "refuses a fixup! with a Generated-with footer" \
	refuses 'fixup! nef: Fix it\n\nGenerated with Claude Code\n'
check "refuses a merge naming a tool as co-author" \
	refuses "Merge branch 'wip/topic'\n\nCo-authored-by: Copilot <copilot@example.invalid>\n"
check "judges 'Revert stuff.' as a person's subject" refuses 'Revert stuff.\n'
check "judges 'Merge everything, lowercase.' as a person's subject" \
	refuses 'Merge everything, lowercase.\n'

echo '# The message as git will store it'
check "refuses a period hidden by a trailing space" refuses 'nef: Find the preview. \n'
check "judges the first line after a leading blank line" refuses '\nnef: find the preview\n'
check "judges the first line after a leading comment" \
	refuses '# Please enter the commit message.\nnef: find the preview\n'
check "accepts a second line holding only spaces" accepts 'nef: Fix it\n  \nBody.\n'
check "ignores everything below the scissors line" \
	accepts 'nef: Fix it\n\nBody.\n# ------------------------ >8 ------------------------\n-Co-Authored-By: Claude <noreply@anthropic.com>\n'
check "accepts a message of comments only (git aborts it)" accepts '# Please enter the commit message.\n'
check "drops a leading # line at commit time, as the editor does" accepts '#12 fix stuff.\n'
check "judges a stored # line as the subject" stored_refuses '#12 fix stuff.\n'
check "  and a stored scissors line as content" \
	stored_refuses 'nef: Fix it\n\n# ------------------------ >8 ------------------------\nCo-Authored-By: Claude <noreply@anthropic.com>\n'

echo '# A tag glued to its summary'
check "refuses 'ui:fix the thing'" refuses 'ui:fix the thing\n'
check "refuses 'nef:Find the preview'" refuses 'nef:Find the preview\n'
check "refuses 'fix(ui):thing happened'" refuses 'fix(ui):thing happened\n'
check "refuses a tab after the tag" refuses 'nef:\tfind the preview\n'
check "accepts an Exif tag name" accepts 'EXIF:Orientation is honoured for previews\n'
check "accepts a maker's tag name" accepts 'Nikon:ShutterCount is read from the MakerNote\n'
check "accepts a path identifier" accepts 'Vec::new no longer allocates here\n'
check "accepts a URL" accepts 'https://raw.pixls.us is where the samples come from\n'
check "accepts a time" accepts '10:30 is when the job runs\n'
check "accepts a ratio" accepts '16:9 frames keep their crop\n'
check "accepts a drive letter" accepts 'C:\\Users paths are read as given\n'
check "accepts a file and line" accepts 'src/lib.rs:42 checks the offset first\n'

echo '# A lower-case first word with punctuation'
check "refuses \"don't do this\"" refuses "don't do this\n"
# \0342\0200\0231 is U+2019, the typographic apostrophe, in UTF-8.
check "refuses 'don't do this' with a typographic apostrophe" \
	refuses 'don\0342\0200\0231t do this\n'
check "refuses 'add, remove things'" refuses 'add, remove things\n'
check "refuses 'fix. the thing' after a tag" refuses 'nef: fix. the thing\n'
check "accepts a hyphenated crate name" accepts 'raw-files builds on Windows\n'

echo '# The same answers in any locale'
# A Turkish locale folds I and i differently. These cases bite only where
# one is installed; elsewhere they are skipped.
tr_locale="$(locale -a 2> /dev/null | grep -m 1 -i '^tr_TR' || true)"
if [ -n "$tr_locale" ]; then
	turkish() { LC_ALL="$tr_locale" "$@"; }
	check "refuses 'FIXES: two' in $tr_locale" turkish refuses 'nef: Fix it\n\nBody.\n\nFIXES: two\n'
	check "refuses a lower-case summary in $tr_locale" turkish refuses 'nef: find the preview\n'
	check "refuses 'ui:fix the thing' in $tr_locale" turkish refuses 'ui:fix the thing\n'
else
	echo "SKIP  no Turkish locale installed"
fi

echo '# Installed in a repository'
C="$W/clone"
git init -q -b master "$C"
git -C "$C" config core.hooksPath "$SRC/.githooks"
# Refused by the hook, not by anything else that can fail a commit.
commit_refused() {
	local out
	! out="$(git -C "$C" commit -q --allow-empty -m "$1" 2>&1)" &&
		grep -q '^commit-msg: ' <<< "$out"
}
check "a commit with a glued tag is refused" commit_refused 'ui:fix the thing'
check "a commit with a good message goes through" \
	git -C "$C" commit -q --allow-empty -m 'nef: Find the preview'

echo
if [ "$FAILS" -eq 0 ]; then
	echo "all passed"
else
	echo "$FAILS check(s) FAILED"
	exit 1
fi
