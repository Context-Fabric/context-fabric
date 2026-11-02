#!/usr/bin/env bash
# Test of adding a dictionary in the standalone program: start it as a server
# (no window) with an empty home folder, add BDB from the library, and check
# that it is there with its licence and an entry. This downloads about 4.6 MB
# from the release of Context-Fabric/bdb, and checks what the frozen program
# needs for it: certificates for HTTPS, the checksum, and writing into the
# home folder. Needs bash and curl.
#
#     packaging/dictionary-test.sh dist/cfabric-browser/cfabric-browser

set -u
program=$1
port=${2:-8199}
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
  [ -n "${GITHUB_ACTIONS:-}" ] && echo "::error title=Dictionary test::$*"
  log=$home/.cfabric/logs/cfabric-browser.log
  [ -f "$log" ] && { echo "--- $log"; tail -40 "$log"; }
  stop
  exit 1
}

"$program" --no-window --no-open --port "$port" &
pid=$!

start=$(date +%s)
until curl -s -o /dev/null "$api/dictionaries"; do
  kill -0 "$pid" 2>/dev/null || fail "the program stopped"
  [ $(($(date +%s) - start)) -gt 120 ] && fail "no answer after 120 s"
  sleep 1
done
echo "server answers after $(($(date +%s) - start)) s"

list=$(curl -s "$api/dictionaries")
echo "$list" | grep -q '"id":"bdb"' || fail "BDB is not offered: ${list:0:300}"
echo "$list" | grep -q '"available":true' || fail "BDB can't be downloaded"
curl -s -X POST "$api/dictionaries/bdb" | grep -q '"state":"running"' || fail "adding did not start"

start=$(date +%s)
until entry=$(curl -s "$api/dictionaries/bdb") && ! echo "$entry" | grep -q '"state":"running"'; do
  [ $(($(date +%s) - start)) -gt 300 ] && fail "adding did not finish in 300 s"
  sleep 2
done
echo "$entry" | grep -q '"state":"done"' || fail "adding failed: $(echo "$entry" | grep -oE '"error":"[^"]*"')"
echo "$entry" | grep -q '"licence":"CC BY-NC 4.0"' || fail "no licence in its metadata"
echo "added after $(($(date +%s) - start)) s"

# בָּרָא: its text, with a verse as an OSIS reference
article=$(curl -s "$api/dictionaries/bdb/entries/b.cw.aa")
echo "$article" | grep -q 'data-osis=\\"Gen.1.1\\"' || fail "the entry of ברא is not shown: ${article:0:300}"
[ -f "$home/.cfabric/dictionaries/bdb/bdb.json.gz" ] || fail "the file is not in the home folder"

curl -s -X DELETE "$api/dictionaries/bdb" | grep -q '"added":false' || fail "could not remove it"

stop
rm -rf "$home"
echo "OK"
