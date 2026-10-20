// CACCHT Explorer for Claude Desktop: a small proxy that passes
// each MCP message (a line of JSON on stdin) on to the running browser's /mcp
// and writes its answer as a line on stdout. The browser's /mcp is stateless
// and answers with JSON, so nothing else is needed; it is found in
// ~/.cfabric/running.json, which the browser writes when it starts. The same
// as `cfabric-browser mcp` (cfabric_browser/assistant/stdio.py).

const fs = require("fs");
const os = require("os");
const path = require("path");
const http = require("http");
const readline = require("readline");

const RUNNING = path.join(os.homedir(), ".cfabric", "running.json");

function post(url, body, timeout) {
  return new Promise((resolve, reject) => {
    const request = http.request(
      url,
      {
        method: body === null ? "GET" : "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json, text/event-stream" },
        timeout,
      },
      (response) => {
        const chunks = [];
        response.on("data", (c) => chunks.push(c));
        response.on("end", () => resolve({ status: response.statusCode, body: Buffer.concat(chunks).toString("utf8") }));
      },
    );
    request.on("timeout", () => request.destroy(new Error("timeout")));
    request.on("error", reject);
    if (body !== null) request.write(body);
    request.end();
  });
}

// the newest running browser that answers
async function findBrowser() {
  let browsers = [];
  try {
    browsers = JSON.parse(fs.readFileSync(RUNNING, "utf8")).browsers || [];
  } catch {
    return null;
  }
  for (const entry of browsers.slice().reverse()) {
    try {
      const answer = await post(entry.url + "api/health", null, 3000);
      if (answer.status === 200) return entry.url;
    } catch {
      // not this one
    }
  }
  return null;
}

function error(message, text) {
  if (message.id === undefined) return null;
  return JSON.stringify({ jsonrpc: "2.0", id: message.id, error: { code: -32603, message: text } });
}

let url = null;

async function handle(line) {
  let message;
  try {
    message = JSON.parse(line);
  } catch {
    return null;
  }
  for (let attempt = 0; attempt < 2; attempt++) {
    if (url === null) url = await findBrowser();
    if (url === null) {
      return error(message, "CACCHT Explorer is not running. Start it, then ask again.");
    }
    try {
      const answer = await post(url + "mcp", line, 180000);
      if (answer.status >= 400) return error(message, `The browser answered ${answer.status}`);
      return answer.body.trim() ? answer.body.trim() : null;
    } catch {
      // it stopped, or restarted on another port: look again
      url = null;
    }
  }
  return error(message, "Lost the connection to the browser");
}

// one message at a time, in order
let queue = Promise.resolve();
readline.createInterface({ input: process.stdin }).on("line", (line) => {
  if (!line.trim()) return;
  queue = queue.then(async () => {
    const answer = await handle(line);
    if (answer !== null) process.stdout.write(answer + "\n");
  });
});
