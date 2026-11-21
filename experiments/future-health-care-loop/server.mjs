import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
const dir = dirname(fileURLToPath(import.meta.url));
const allowed = new Map([
  ["/", ["index.html", "text/html; charset=utf-8"]],
  ["/src/care-loop.mjs", ["src/care-loop.mjs", "text/javascript; charset=utf-8"]],
  ["/fixtures/synthetic.mjs", ["fixtures/synthetic.mjs", "text/javascript; charset=utf-8"]],
]);
const port = 8787;
createServer(async (req, res) => {
  if (req.method !== "GET" || !allowed.has(req.url)) {
    res.writeHead(404, {"Content-Type":"text/plain", "X-Content-Type-Options":"nosniff"});
    res.end("Not found");
    return;
  }
  const [file, type] = allowed.get(req.url);
  try {
    const content = await readFile(join(dir, file));
    res.writeHead(200, {
      "Content-Type":type,
      "Cache-Control":"no-store",
      "X-Content-Type-Options":"nosniff",
      "Content-Security-Policy":"default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; connect-src 'none'; img-src 'none'; base-uri 'none'",
    });
    res.end(content);
  } catch {
    res.writeHead(500, {"Content-Type":"text/plain"});
    res.end("Local file unavailable");
  }
}).listen(port, "127.0.0.1", () => {
  process.stdout.write("Synthetic Care Loop at http://127.0.0.1:" + port + "\n");
});