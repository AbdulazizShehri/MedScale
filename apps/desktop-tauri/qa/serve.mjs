// Serves dist/ with the QA replay injected. Usage: node qa/serve.mjs <fixtures.json> [port]
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
const root = path.resolve("dist"), fixtures = process.argv[2], port = Number(process.argv[3] ?? 47124);
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".ttf": "font/ttf", ".svg": "image/svg+xml", ".png": "image/png" };
http.createServer((req, res) => {
  let p = decodeURIComponent(req.url.split("?")[0]);
  let file = p === "/" ? path.join(root, "index.html") : p === "/qa-mock.js" ? path.resolve("qa/mock-tauri.js") : p === "/ui-fixtures.json" ? fixtures : path.join(root, p);
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); res.end(); return; }
    if (file.endsWith("index.html")) data = Buffer.from(data.toString().replace("<script type=\"module\"", "<script src=\"/qa-mock.js\"></script><script type=\"module\""));
    res.writeHead(200, { "content-type": types[path.extname(file)] ?? "application/octet-stream" });
    res.end(data);
  });
}).listen(port, "127.0.0.1", () => console.log("qa on " + port));
