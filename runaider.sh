#!/bin/bash

# Build the aider command
cmd=("aider")

# Add --read-only for each file argument
for file in "$@"; do
    cmd+=("--read-only" "$file")
done

# Always add these flags
cmd+=("--dark" "--vim")

# Execute the command
exec "${cmd[@]}"
