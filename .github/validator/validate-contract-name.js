module.exports = async (field) => {
  if (!field || typeof field !== 'string') return 'success';
  const value = field.trim();
  if (value.length === 0) return 'success';
  // Optional `<prefix>/` then a kebab-or-snake segment. Each segment must
  // start with a lowercase letter and contain only [a-z0-9_-]. Length capped
  // at 64 chars total to match the registry's name storage policy.
  const re = /^([a-z][a-z0-9_-]*\/)?[a-z][a-z0-9_-]*$/;
  if (!re.test(value)) {
    return 'name must be kebab- or snake-case (lowercase letters, digits, `_`, `-`), optionally prefixed with `<subregistry>/`. Examples: `hello`, `unverified/my-thing`, `oz/fungible_token`.';
  }
  if (value.length > 64) {
    return `name exceeds 64 characters (current: ${value.length}).`;
  }
  return 'success';
};
