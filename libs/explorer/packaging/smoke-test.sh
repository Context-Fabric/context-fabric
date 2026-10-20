#!/usr/bin/env bash
# Smoke test of the standalone program: start it as a server (no window) with a
# small corpus, and check that the interface is served and that a search runs in
# its worker processes. Needs only bash and curl, not Python.
#
#     packaging/smoke-test.sh dist/cfabric-browser/cfabric-browser tests/fixtures/display_corpus

set -u
program=$1
corpus=$2
port=${3:-8199}
api=http://127.0.0.1:$port/api

stop() {
  if [ -r "/proc/$pid/winpid" ]; then
    # Git Bash on Windows: kill does not reliably reach a Windows program, so end
    # the program and its search workers with taskkill
    taskkill //F //T //PID "$(cat "/proc/$pid/winpid")" > /dev/null
  else
    kill "$pid" 2>/dev/null # a normal stop, which also stops the workers
  fi
  wait "$pid" 2>/dev/null
}

fail() {
  echo "FAIL: $*"
  # in GitHub Actions also as an annotation, which can be read without a login
  [ -n "${GITHUB_ACTIONS:-}" ] && echo "::error title=Smoke test::$*"
  stop
  exit 1
}

"$program" --no-window --no-open --port "$port" "test=$corpus" &
pid=$!

start=$(date +%s)
until curl -s -o /dev/null "$api/corpora"; do
  kill -0 "$pid" 2>/dev/null || fail "the program stopped"
  [ $(($(date +%s) - start)) -gt 120 ] && fail "no answer after 120 s"
  sleep 1
done
echo "server answers after $(($(date +%s) - start)) s"

curl -s "$api/corpora" | grep -q '"name":"test"' || fail "corpus not loaded"
curl -s "http://127.0.0.1:$port/" | grep -q '<div id="root">' || fail "no interface"

job=$(curl -s -X POST "$api/corpora/test/search" \
  -H 'content-type: application/json' -d '{"template":"word"}' |
  grep -oE '"id":"[^"]+"' | cut -d'"' -f4)
[ -n "$job" ] || fail "search not started"
start=$(date +%s)
until status=$(curl -s "$api/searches/$job") &&
  echo "$status" | grep -qE '"state":"(done|failed|timeout|cancelled)"'; do
  [ $(($(date +%s) - start)) -gt 60 ] && fail "search did not finish"
  sleep 1
done
echo "$status" | grep -q '"state":"done"' || fail "search: $status"
echo "search: $(echo "$status" | grep -oE '"count":[0-9]+')"

stop
echo "OK"
