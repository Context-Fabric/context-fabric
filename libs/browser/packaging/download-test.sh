#!/usr/bin/env bash
# Test of downloading a corpus in the standalone program: start it as a server
# (no window) with an empty home folder, check that it offers the missing
# corpora at the start, download the SP (4 MB, the smallest) as the start screen
# does, and check that it is prepared and opens. Needs bash and curl.
#
#     packaging/download-test.sh dist/cfabric-browser/cfabric-browser

set -u
program=$1
port=${2:-8197}
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
  [ -n "${GITHUB_ACTIONS:-}" ] && echo "::error title=Download test::$*"
  log=$home/.cfabric/logs/cfabric-browser.log
  [ -f "$log" ] && { echo "--- $log"; tail -40 "$log"; }
  stop
  exit 1
}

"$program" --no-window --no-open --port "$port" &
pid=$!

start=$(date +%s)
until curl -s -o /dev/null "$api/health"; do
  kill -0 "$pid" 2>/dev/null || fail "the program stopped"
  [ $(($(date +%s) - start)) -gt 120 ] && fail "no answer after 120 s"
  sleep 1
done
echo "server answers after $(($(date +%s) - start)) s"

curl -s "$api/health" | grep -q '"askDownloads":true' || fail "the program does not offer downloads at the start"
list=$(curl -s "$api/downloads")
for id in ETCBC/bhsa DT-UCPH/sp ETCBC/dss; do
  echo "$list" | grep -q "\"id\":\"$id\"" || fail "$id is not offered"
done
echo "$list" | grep -q '"present":true' && fail "a corpus counts as present in an empty home folder"

curl -s -X POST "$api/downloads/DT-UCPH/sp" | grep -q '"state":"running"' || fail "the download did not start"
start=$(date +%s)
until entry=$(curl -s "$api/downloads/DT-UCPH/sp") && ! echo "$entry" | grep -q '"state":"running"'; do
  [ $(($(date +%s) - start)) -gt 600 ] && fail "the download did not finish in 600 s"
  sleep 2
done
echo "$entry" | grep -q '"state":"done"' || fail "the download failed: $(echo "$entry" | grep -oE '"error":"[^"]*"')"
echo "$entry" | grep -q '"present":true' || fail "downloaded, but not there"
echo "downloaded and prepared after $(($(date +%s) - start)) s"

# prepared means open and loaded
curl -s "$api/corpora" | grep -q '"name":"sp"' || fail "the SP is not open after downloading"
passage=$(curl -s "$api/corpora/sp/passage?ref=Genesis+1&mode=plain")
echo "$passage" | grep -q '"trees"' || fail "Genesis 1 not shown: ${passage:0:300}"
[ -f "$home/text-fabric-data/github/DT-UCPH/sp/tf/7.2.4/__checkout__.txt" ] || fail "no __checkout__.txt"

stop
rm -rf "$home"
echo "OK"
