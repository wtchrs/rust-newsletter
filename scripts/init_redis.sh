#!/usr/bin/env bash
set -x
set -eo pipefail

RUNNING_CONTAINER=$(docker ps --filter 'name=redis' --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
  echo >&2 "Error: redis container is already running, kill it with"
  echo >&2 "  docker kill $RUNNING_CONTAINER"
  exit 1
fi

docker run -d -p 6379:6379 --name "redis_$(date '+%s')" redis

echo >&2 "redis is up and running on port 6379."
