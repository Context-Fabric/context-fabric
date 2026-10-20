#!/usr/bin/env bash
# Test of adding a translation in the standalone program: start it as a server
# (no window) with an empty home folder, add the JPS 1917 from the library, and
# check that it is built and opens. This downloads about 5 MB from Sefaria, and
# checks what the frozen program needs for it: certificates for HTTPS, the data
# files of cfabric_browser, and writing into the home folder. Needs bash and curl.
#
#     packaging/translation-test.sh dist/cfabric-browser/cfabric-browser

set -u
program=$1
port=${2:-8198}
api=http://127.0.0.1:$port/api

home=$(mktemp -d)
if command -v cygpath > /dev/null; then
  # Git Bash on Windows: the program finds the home folder by USERPROFILE
  export USERPROFILE=$(cygpath -w "$home")
fi
export HOME=$home

stop() {
  if [ -r "/proc/$pid/winpid" ]; then
    taskkill //F //T //PID "$(cat "/proc/$pid/winpid")" > /dev/null
  else
    kill "$pid" 2>/dev/null
  fi
  wait "$pid" 2>/dev/null
}

fail() {
  echo "FAIL: $*"
  [ -n "${GITHUB_ACTIONS:-}" ] && echo "::error title=Translation test::$*"
  log=$home/.cfabric/logs/cfabric-browser.log
  [ -f "$log" ] && { echo "--- $log"; tail -40 "$log"; }
  stop
  exit 1
}

"$program" --no-window --no-open --port "$port" &
pid=$!

start=$(date +%s)
until curl -s -o /dev/null "$api/translations"; do
  kill -0 "$pid" 2>/dev/null || fail "the program stopped"
  [ $(($(date +%s) - start)) -gt 120 ] && fail "no answer after 120 s"
  sleep 1
done
echo "server answers after $(($(date +%s) - start)) s"

curl -s "$api/translations" | grep -q '"id":"jps1917"' || fail "the JPS 1917 is not offered"
curl -s -X POST "$api/translations/jps1917" | grep -q '"state":"running"' || fail "adding did not start"

start=$(date +%s)
until entry=$(curl -s "$api/translations/jps1917") && ! echo "$entry" | grep -q '"state":"running"'; do
  [ $(($(date +%s) - start)) -gt 300 ] && fail "adding did not finish in 300 s"
  sleep 2
done
echo "$entry" | grep -q '"state":"done"' || fail "adding failed: $(echo "$entry" | grep -oE '"error":"[^"]*"')"
echo "added after $(($(date +%s) - start)) s"

curl -s -X POST "$api/corpora" -H 'content-type: application/json' \
  -d '{"id":"Context-Fabric/jps1917"}' | grep -q '"name":"jps1917"' || fail "could not open it"
# the first load compiles the corpus
start=$(date +%s)
until passage=$(curl -s "$api/corpora/jps1917/passage?ref=Malachi+3&mode=plain") &&
  echo "$passage" | grep -q '"notes"'; do
  [ $(($(date +%s) - start)) -gt 180 ] && fail "Malachi 3 not shown: ${passage:0:300}"
  sleep 2
done
echo "$passage" | grep -q 'Behold, I will send you Elijah' || fail "the note after Malachi 3:24 is missing"
echo "opened after $(($(date +%s) - start)) s"

stop
rm -rf "$home"
echo "OK"
