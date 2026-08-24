import { Buffer } from "node:buffer";

const encoder = new TextEncoder();
const MAX_CLOCK_SKEW_MS = 5 * 60 * 1000;
const LEASE_MS = 2 * 60 * 1000;

export class WalletSession {
  constructor(state) {
    this.state = state;
  }

  async fetch(request) {
    const body = await request.json();
    const path = new URL(request.url).pathname;
    const current = await this.state.storage.get("writer");
    if (path === "/claim") {
      if (current && body.generation < current.generation) return Response.json({ error: "stale generation" }, { status: 409 });
      if (current && body.generation === current.generation &&
          (body.backupHashHex !== current.backupHashHex || body.devicePublicKeyHex !== current.devicePublicKeyHex)) {
        return Response.json({ error: "generation conflict" }, { status: 409 });
      }
      await this.state.storage.put("writer", body);
      return Response.json({ accepted: true });
    }
    if (!current || current.generation !== body.generation || current.backupHashHex !== body.backupHashHex || current.devicePublicKeyHex !== body.devicePublicKeyHex) {
      return Response.json({ error: "stale lease" }, { status: 409 });
    }
    if (path === "/submit/get") {
      return Response.json({ transactionId: await this.state.storage.get(`operation:${body.operationId}`) ?? null });
    }
    if (path === "/submit/put") {
      const key = `operation:${body.operationId}`;
      const existing = await this.state.storage.get(key);
      if (existing && existing !== body.transactionId) return Response.json({ error: "operation conflict" }, { status: 409 });
      await this.state.storage.put(key, body.transactionId);
      return Response.json({ transactionId: body.transactionId });
    }
    return Response.json({ accepted: true });
  }
}

export async function endpointSessionClaim(ctx) {
  const body = await ctx.req.json();
  try { validateClaim(body); } catch (error) { return ctx.json({ error: error.message }, 400); }
  if (Math.abs(Date.now() - body.timestamp) > MAX_CLOCK_SKEW_MS) return ctx.json({ error: "stale timestamp" }, 400);
  if (!(await verifyClaimSignature(body))) return ctx.json({ error: "invalid signature" }, 401);

  const object = sessionObject(ctx, body.walletVerificationKeyHex);
  const accepted = await object.fetch("https://session/claim", { method: "POST", body: JSON.stringify(body) });
  if (!accepted.ok) return new Response(accepted.body, { status: accepted.status, headers: accepted.headers });

  const expiresAtEpochMillis = Date.now() + LEASE_MS;
  const payload = {
    walletVerificationKeyHex: body.walletVerificationKeyHex,
    generation: body.generation,
    backupHashHex: body.backupHashHex,
    devicePublicKeyHex: body.devicePublicKeyHex,
    expiresAtEpochMillis,
  };
  return ctx.json({ lease: await signToken(payload, ctx.env.SESSION_SECRET), expiresAtEpochMillis });
}

export async function endpointSessionCheck(ctx) {
  return (await requireLease(ctx)) ? ctx.json({ valid: true }) : ctx.json({ valid: false }, 401);
}

export async function requireLease(ctx) {
  const token = ctx.req.header("FERRET-SESSION");
  if (!token) return null;
  const payload = await verifyToken(token, ctx.env.SESSION_SECRET);
  if (!payload || payload.expiresAtEpochMillis <= Date.now()) return null;
  const object = sessionObject(ctx, payload.walletVerificationKeyHex);
  const response = await object.fetch("https://session/verify", { method: "POST", body: JSON.stringify(payload) });
  return response.ok ? { payload, object } : null;
}

function sessionObject(ctx, walletVerificationKeyHex) {
  const id = ctx.env.WALLET_SESSIONS.idFromName(walletVerificationKeyHex);
  return ctx.env.WALLET_SESSIONS.get(id);
}

function validateClaim(body) {
  if (!Number.isSafeInteger(body.generation) || body.generation < 1) throw new Error("invalid generation");
  for (const [name, value, bytes] of [
    ["walletVerificationKeyHex", body.walletVerificationKeyHex, 32],
    ["backupHashHex", body.backupHashHex, 32],
    ["devicePublicKeyHex", body.devicePublicKeyHex, 32],
    ["signatureHex", body.signatureHex, 64],
  ]) if (typeof value !== "string" || !new RegExp(`^[0-9a-f]{${bytes * 2}}$`).test(value)) throw new Error(`invalid ${name}`);
  if (!Number.isSafeInteger(body.timestamp)) throw new Error("invalid timestamp");
}

async function verifyClaimSignature(body) {
  const key = await crypto.subtle.importKey("raw", hex(body.walletVerificationKeyHex), { name: "Ed25519" }, false, ["verify"]);
  return crypto.subtle.verify("Ed25519", key, hex(body.signatureHex), encoder.encode(claimMessage(body)));
}

function claimMessage(body) {
  return `${body.generation}\n${body.backupHashHex}\n${body.devicePublicKeyHex}\n${body.timestamp}`;
}

async function signToken(payload, secret) {
  const encoded = base64url(encoder.encode(JSON.stringify(payload)));
  return `${encoded}.${base64url(await hmac(secret, encoded))}`;
}

async function verifyToken(token, secret) {
  const parts = token.split(".");
  if (parts.length !== 2) return null;
  const expected = await hmac(secret, parts[0]);
  const actual = base64urlDecode(parts[1]);
  if (!constantTimeEqual(actual, new Uint8Array(expected))) return null;
  try { return JSON.parse(new TextDecoder().decode(base64urlDecode(parts[0]))); } catch { return null; }
}

async function hmac(secret, value) {
  const key = await crypto.subtle.importKey("raw", encoder.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  return crypto.subtle.sign("HMAC", key, encoder.encode(value));
}

function constantTimeEqual(left, right) {
  if (left.length !== right.length) return false;
  let difference = 0;
  for (let i = 0; i < left.length; i++) difference |= left[i] ^ right[i];
  return difference === 0;
}

function hex(value) {
  return Uint8Array.from(value.match(/../g), byte => Number.parseInt(byte, 16));
}

function base64url(bytes) {
  return Buffer.from(bytes).toString("base64url");
}

function base64urlDecode(value) {
  return new Uint8Array(Buffer.from(value, "base64url"));
}
