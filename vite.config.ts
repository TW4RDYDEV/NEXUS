import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import path from "node:path";
function localRustBridge(): Plugin {
  return {
    name: "nexus-local-rust-bridge",
    apply: "serve",
    configureServer(server) {
      if (!process.env.NEXUS_BRIDGE) return;
      const binary = path.resolve(process.env.NEXUS_BRIDGE);
      const child = spawn(binary, [], {
        stdio: ["pipe", "pipe", "inherit"],
        windowsHide: true,
        env: process.env,
      });
      let sequence = 0;
      const pending = new Map<
        number,
        { resolve: (v: unknown) => void; reject: (e: Error) => void }
      >();
      createInterface({ input: child.stdout }).on("line", (line) => {
        try {
          const data = JSON.parse(line);
          const p = pending.get(data.id);
          if (p) {
            pending.delete(data.id);
            p.resolve(data);
          }
        } catch {
          for (const p of pending.values())
            p.reject(new Error("Invalid backend response"));
          pending.clear();
        }
      });
      child.on("error", (error) => {
        for (const p of pending.values()) p.reject(error);
        pending.clear();
      });
      child.on("exit", () => {
        for (const p of pending.values())
          p.reject(new Error("Rust backend stopped"));
        pending.clear();
      });
      server.httpServer?.on("close", () => child.kill());
      server.middlewares.use("/__nexus", async (req, res) => {
        res.setHeader("Content-Type", "application/json");
        const origin = req.headers.origin;
        const host = req.headers.host;
        if (
          req.method !== "POST" ||
          req.headers["x-nexus-local"] !== "1" ||
          (origin && origin !== `http://${host}`)
        ) {
          res.statusCode = 403;
          res.end(JSON.stringify({ error: "Local same-origin requests only" }));
          return;
        }
        try {
          const chunks: Buffer[] = [];
          let size = 0;
          for await (const chunk of req) {
            size += chunk.length;
            if (size > 50 * 1024 * 1024) throw Error("Request too large");
            chunks.push(chunk);
          }
          const request = JSON.parse(Buffer.concat(chunks).toString());
          const id = ++sequence;
          const result = await new Promise((resolve, reject) => {
            pending.set(id, { resolve, reject });
            child.stdin.write(
              JSON.stringify({ id, op: request.op, args: request.args ?? {} }) +
                "\n",
            );
          });
          res.end(JSON.stringify(result));
        } catch (e) {
          res.statusCode = 500;
          res.end(
            JSON.stringify({
              error: e instanceof Error ? e.message : "Backend request failed",
            }),
          );
        }
      });
    },
  };
}
export default defineConfig({
  plugins: [react(), localRustBridge()],
  server: { port: 1420, strictPort: true, host: "127.0.0.1" },
  clearScreen: false,
});
