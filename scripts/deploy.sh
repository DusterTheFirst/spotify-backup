#!/bin/sh

set -eu -o pipefail

# Check git status for untracked/modified files
if [[ -n $(git status --porcelain=v1) ]]; then
    echo "git working tree is dirty, exiting.";
    git status --short

    exit 126;
fi;

# Prompt user for confirmation
read -e -p "This will create and deploy a release. are you sure you wish to do continue? [y/N] " choice
if [[ "$choice" != [Yy]* ]]; then
    echo "aborting.";

    exit 127;
fi;

# Start printing out all commands as they run
set -x

SENTRY_VERSION=$(sentry-cli releases propose-version)
sentry-cli releases new "$SENTRY_VERSION" --org "$SENTRY_ORGANIZATION_SLUG" --project "$SENTRY_PROJECT_SLUG"

# TODO: cargo release/use semver release?
# TODO: cargo deny
flyctl deploy --remote-only

sentry-cli releases finalize "$SENTRY_VERSION" --org "$SENTRY_ORGANIZATION_SLUG" --project "$SENTRY_PROJECT_SLUG"