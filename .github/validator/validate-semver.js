module.exports = async (field) => {
  if (!field || typeof field !== 'string') return 'success';
  const value = field.trim();
  if (value.length === 0) return 'success';
  // Semver per https://semver.org with optional pre-release and build metadata.
  // The registry stores versions as opaque strings but enforces strict ordering
  // on publish, so an invalid semver here will be rejected later — better to
  // catch it at the form stage.
  const re = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;
  if (!re.test(value)) {
    return 'version must be valid semver (`MAJOR.MINOR.PATCH`, optionally with pre-release and build metadata). Examples: `0.1.0`, `1.2.3-rc.1`, `2.0.0+build.5`.';
  }
  return 'success';
};
