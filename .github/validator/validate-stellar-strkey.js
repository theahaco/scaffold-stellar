module.exports = async (field) => {
  if (!field || typeof field !== 'string') return 'success';
  const value = field.trim();
  if (value.length === 0) return 'success';
  // Stellar strkey: 56 chars, base32 alphabet (A-Z, 2-7), version byte
  // determines kind. We accept G (ed25519 account) or C (contract).
  // Muxed M and signed-payload P are intentionally rejected for these fields —
  // the registry methods take account/contract addresses, not muxed.
  const re = /^[GC][A-Z2-7]{55}$/;
  if (!re.test(value)) {
    return 'must be a 56-character Stellar strkey starting with `G` (account) or `C` (contract). Example: `GABC…56-chars` or `CABC…56-chars`.';
  }
  return 'success';
};
