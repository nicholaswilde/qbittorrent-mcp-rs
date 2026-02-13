#!/bin/bash
# Test range detection logic

TAGS=$(git tag --sort=-v:refname | head -n 2)
LATEST=$(echo "$TAGS" | sed -n '1p')
PREVIOUS=$(echo "$TAGS" | sed -n '2p')

echo "Latest: $LATEST"
echo "Previous: $PREVIOUS"

if [ -z "$LATEST" ] || [ -z "$PREVIOUS" ]; then
    echo "Error: Could not find two tags."
    exit 1
fi

RANGE="$PREVIOUS..$LATEST"
echo "Detected Range: $RANGE"

# Verify range works with git log
git log --oneline "$RANGE" > /dev/null
if [ $? -eq 0 ]; then
    echo "Range is valid."
else
    echo "Range is invalid."
    exit 1
fi
