#!/usr/bin/env bash
# Test of the program as an MCP server for an AI app: start it as a server (no
# window) with an empty home folder, then start it a second time as
# `<program> mcp`, as Claude Desktop does, and ask for its tools and a trial
# search over stdio. Checks that the MCP SDK made it into the frozen program,
# that `mcp` keeps stdout for its messages, and that the helper finds the
# browser through ~/.cfabric/running.json. Needs bash and curl.
#
#     packaging/mcp-test.sh dist/cfabric-browser/cfabric-browser

set -u
program=$1
port=${2:-8197}
api=http://127.0.0.1:$port/api

home=$(mktemp -d)
if command -v cygpath > /dev/null; then
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
  [ -n "${GITHUB_ACTIONS:-}" ] && echo "::error title=MCP test::$*"
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
[ -f "$home/.cfabric/running.json" ] || fail "no ~/.cfabric/running.json"

# the browser itself, over HTTP
answer=$(curl -s -X POST "http://127.0.0.1:$port/mcp" \
  -H 'content-type: application/json' -H 'accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}')
echo "$answer" | grep -q '"propose_search"' || fail "/mcp has no tools: ${answer:0:300}"
# not from a web page
code=$(curl -s -o /dev/null -w '%{http_code}' -X POST "http://127.0.0.1:$port/mcp" \
  -H 'content-type: application/json' -H 'origin: http://example.org' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}')
[ "$code" -ge 400 ] || fail "/mcp answered a web page ($code)"

# as an AI app starts it: the program with `mcp`, over stdio
out=$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_corpora","arguments":{}}}' \
  | timeout 120 "$program" mcp)
echo "$out" | grep -q '"serverInfo"' || fail "no answer to initialize over stdio: ${out:0:300}"
echo "$out" | grep -q '"try_search"' || fail "no tools over stdio: ${out:0:300}"
echo "$out" | grep -q '"id":3' || fail "list_corpora gave no answer: ${out:0:300}"
lines=$(echo "$out" | grep -c '"jsonrpc"')
[ "$lines" -eq 3 ] || fail "expected 3 answers on stdout, got $lines: ${out:0:500}"
echo "stdio: initialize, tools/list and a tool call answered"

stop
rm -rf "$home"
echo "OK"
