import { createHash, createPrivateKey, createPublicKey, sign as nodeSign, verify as nodeVerify } from "node:crypto";
import { b58encode, b58decode } from "./base58.js";

export const SIGNATURE_PREFIX = "ed25519:";
const PRIVATE_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");
const PUBLIC_PREFIX = Buffer.from("302a300506032b6570032100", "hex");
export function sha256Hex(value: string | Uint8Array): string { return createHash("sha256").update(value).digest("hex"); }
function asBytes(value: string | Uint8Array): Buffer { return typeof value === "string" ? Buffer.from(value) : Buffer.from(value); }
export class Ed25519Signer {
  private constructor(private readonly seed: Buffer, private readonly privateKey: ReturnType<typeof createPrivateKey>, private readonly publicKey: Buffer) {}
  static fromSeed(seed: string): Ed25519Signer { const bytes = createHash("sha256").update(seed).digest(); const privateKey = createPrivateKey({ key: Buffer.concat([PRIVATE_PREFIX, bytes]), format: "der", type: "pkcs8" }); const publicDer = createPublicKey(privateKey).export({ format: "der", type: "spki" }) as Buffer; return new Ed25519Signer(bytes, privateKey, publicDer.subarray(-32)); }
  sign(message: string | Uint8Array): string { return `${SIGNATURE_PREFIX}${nodeSign(null, asBytes(message), this.privateKey).toString("base64url")}`; }
  publicKeyBytes(): Buffer { return Buffer.from(this.publicKey); }
  verificationMethod(): string { return `did:key:${this.didKey()}#${this.didKey()}`; }
  didKey(): string { return `z${b58encode(Uint8Array.from([0xed, 0x01, ...this.publicKey]))}`; }
}
export function publicKeyFromDidKey(did: string): Buffer { const value = (did.split("#")[0] ?? "").replace(/^did:key:/, ""); const decoded = Buffer.from(b58decode(value.slice(1))); return decoded.subarray(2); }
export function verify(publicKey: string | Uint8Array, message: string | Uint8Array, signature: string): boolean { if (!signature.startsWith(SIGNATURE_PREFIX)) return false; const raw = typeof publicKey === "string" ? publicKeyFromDidKey(publicKey) : Buffer.from(publicKey); const key = createPublicKey({ key: Buffer.concat([PUBLIC_PREFIX, raw]), format: "der", type: "spki" }); return nodeVerify(null, asBytes(message), key, Buffer.from(signature.slice(SIGNATURE_PREFIX.length), "base64url")); }
export function unsignedDigest(message: string | Uint8Array): string { return `unsigned:sha256:${sha256Hex(message)}`; }
