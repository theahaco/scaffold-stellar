module.exports = async (field) => {
  if (!field || typeof field !== 'string') return 'success';
  const value = field.trim();
  if (value.length === 0) return 'success';
  const re = /^https:\/\/[^\s/$.?#].[^\s]*$/i;
  if (!re.test(value)) {
    return 'URL must start with `https://` and be a well-formed URL.';
  }
  return 'success';
};
