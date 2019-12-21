#!/usr/bin/env bash
#
# Takes 2 tags and get all commits between them and turns
# it into a release notes based on AngularJS style
if [ -z "$1" ]; then
    echo "Need a new version" >&2
    exit 1
fi

if [ -z "$2" ]; then
	echo "Need a starting tag" >&2
	exit 1
fi

stag="$2"

if [ "$3" ]; then
	ftag="$3"
else
	ftag=master
fi

commits="$(git log --pretty='%s' ${ftag}...${stag})"

printf "%s\\n\\n" "## Changes in $1"

feats=() # New Features
fixes=() # Fixes

while read -r c; do
    # Don't include reverted commits in changelog
    grep -q '^Revert ' <<< "$c" && continue
	# Add all features to the array
	if grep -q '^feat(.*): ' <<< "$c"; then
		feats+=("${c#*feat}")
		continue
	fi

	if grep -q '^fix(.*): ' <<< "$c"; then
		fixes+=("${c#*fix}")
		continue
	fi
done <<< "$commits"


# Output in XML mode
if [ "$4" == "appdata" ]; then
	if ! [ ${#feats[@]} -eq 0 ]; then
		echo "<ul>"
		for c in "${feats[@]}"; do
			printf "%s\\n" "<li>$c</li>"
		done
		echo "</ul>"
	fi
	if ! [ ${#fixes[@]} -eq 0 ]; then
		echo "<ul>"
		for c in "${fixes[@]}"; do
			printf "%s\\n" "<li>$c</li>"
		done
		echo "</ul>"
	fi
else
	printf "%s\\n\\n" "### Feature changes"

	if [ ${#feats[@]} -eq 0 ]; then
		echo " - No new features"
	else
		for c in "${feats[@]}"; do
			printf "%s\\n" " - $c"
		done
	fi

	echo ""

	printf "%s\\n\\n" "### Bugfixes"

	if [ ${#fixes[@]} -eq 0 ]; then
		echo " - No bugfixes"
	else
		for c in "${fixes[@]}"; do
			printf "%s\\n" " - $c"
		done
	fi
fi
