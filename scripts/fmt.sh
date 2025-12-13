#!/bin/sh

# check for required programs
command -v dirname > /dev/null 2>&1 || { echo >&2 "'dirname' not found"; exit 1; }
command -v cargo > /dev/null 2>&1 || { echo >&2 "'cargo' not found"; exit 1; }
command -v dprint > /dev/null 2>&1 || { echo >&2 "'dprint' not found"; exit 1; }

# go to project root directory
cd -- "$(dirname "${0}")" || exit 1
cd .. || exit 1

# format *.rs files
command='cargo +nightly fmt --all'
printf 'Formatting with `%s`\n' "${command}"
${command} || { echo >&2 'Failed to format code'; exit 1; }

# format *.toml and *.md files
command='dprint fmt'
printf 'Formatting with `%s`\n' "${command}"
${command} || { echo >&2 'Failed to format code'; exit 1; }

# done
echo "Done"
