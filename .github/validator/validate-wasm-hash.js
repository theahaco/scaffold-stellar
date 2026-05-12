module.exports = async (field) => {
  if (!field || typeof field !== 'string') return 'success';
  const value = field.trim();
  if (value.length === 0) return 'success';
  // sha256 — 32 bytes, hex-encoded → 64 lowercase hex chars (case-insensitive
  // accepted; the on-chain registry stores it as bytes regardless).
  const re = /^[0-9a-fA-F]{64}$/;
  if (!re.test(value)) {
    return `wasm hash must be a 64-character hex sha256 (current length: ${value.length}). No \`0x\` prefix. Example: \`a1b2…64-chars\`.`;
  }
  return 'success';
};
