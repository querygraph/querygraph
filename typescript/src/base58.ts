const ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

export function b58encode(bytes: Uint8Array): string {
  let value = 0n;
  for (const byte of bytes) value = (value << 8n) | BigInt(byte);
  let encoded = "";
  while (value > 0n) {
    const remainder = Number(value % 58n);
    encoded = ALPHABET[remainder] + encoded;
    value /= 58n;
  }
  for (const byte of bytes) {
    if (byte !== 0) break;
    encoded = `1${encoded}`;
  }
  return encoded || "1";
}

export function b58decode(text: string): Uint8Array {
  let value = 0n;
  for (const char of text) {
    const index = ALPHABET.indexOf(char);
    if (index < 0) throw new Error(`Invalid base58 character: ${char}`);
    value = value * 58n + BigInt(index);
  }
  const output: number[] = [];
  while (value > 0n) {
    output.unshift(Number(value & 255n));
    value >>= 8n;
  }
  for (const char of text) {
    if (char !== "1") break;
    output.unshift(0);
  }
  return Uint8Array.from(output);
}
