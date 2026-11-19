import { resolve, sep } from "node:path";
import { patchGameGround } from "./ground/patch-game";

const root = import.meta.dir;
const gameRuntime = patchGameGround(
  await Bun.file(resolve(root, "game.js")).text(),
);
const index = await Bun.file(resolve(root, "index.html")).text();
const legacyScript =
  /<script type="application\/octet-stream" id="legacy-v8">([A-Za-z0-9+/=\s]+)<\/script>/;
const legacy = legacyScript.exec(index);
if (!legacy) throw new Error("Embedded Saltline archive is missing.");
const liveHtml = index.replace(
  legacyScript,
  '<script type="application/octet-stream" id="legacy-v8" data-src="legacy-v8.html"></script>',
);
const server = Bun.serve({
  hostname: "127.0.0.1",
  port: Number(process.env.PORT ?? 3000),
  async fetch(request) {
    const url = new URL(request.url);
    if (url.pathname === "/favicon.ico")
      return new Response(null, { status: 204 });
    if (request.method !== "GET" && request.method !== "HEAD") {
      return new Response("Method not allowed", { status: 405 });
    }
    let pathname: string;
    try {
      pathname = decodeURIComponent(url.pathname);
    } catch {
      return new Response("Bad request", { status: 400 });
    }
    if (
      pathname === "/" ||
      pathname === "/index.html" ||
      pathname === "/legacy-v8.html"
    ) {
      const body =
        request.method === "HEAD"
          ? null
          : pathname === "/legacy-v8.html"
            ? Buffer.from(legacy[1], "base64")
            : liveHtml;
      return new Response(body, {
        headers: {
          "Content-Type": "text/html; charset=utf-8",
          "Cache-Control": "no-store",
        },
      });
    }
    if (pathname === "/game.js" && url.searchParams.has("runtime")) {
      return new Response(request.method === "HEAD" ? null : gameRuntime, {
        headers: {
          "Content-Type": "text/javascript; charset=utf-8",
          "Cache-Control": "no-store",
        },
      });
    }
    const path = resolve(root, "." + pathname);
    if (!path.startsWith(root + sep))
      return new Response("Forbidden", { status: 403 });
    const file = Bun.file(path);
    if (!(await file.exists()))
      return new Response("Not found", { status: 404 });
    return new Response(request.method === "HEAD" ? null : file, {
      headers: { "Content-Type": file.type, "Cache-Control": "no-store" },
    });
  },
});

console.log(`Saltline läuft auf ${server.url}`);
