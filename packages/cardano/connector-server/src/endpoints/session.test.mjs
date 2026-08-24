import { describe, expect, it } from "vitest";
import { WalletSession } from "./session.mjs";

class MemoryStorage {
  values = new Map();
  async get(key) { return this.values.get(key); }
  async put(key, value) { this.values.set(key, value); }
}

const claim = (generation, backupHashHex, devicePublicKeyHex) => ({ generation, backupHashHex, devicePublicKeyHex });
const request = (path, body) => new Request(`https://session${path}`, { method: "POST", body: JSON.stringify(body) });

describe("WalletSession", () => {
  it("rejects stale and same-generation divergent writers", async () => {
    const session = new WalletSession({ storage: new MemoryStorage() });
    expect((await session.fetch(request("/claim", claim(2, "aa", "11")))).status).toBe(200);
    expect((await session.fetch(request("/claim", claim(1, "aa", "11")))).status).toBe(409);
    expect((await session.fetch(request("/claim", claim(2, "bb", "22")))).status).toBe(409);
    expect((await session.fetch(request("/claim", claim(3, "bb", "22")))).status).toBe(200);
    expect((await session.fetch(request("/verify", claim(2, "aa", "11")))).status).toBe(409);
  });
});
