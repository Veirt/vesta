// Conventional-commit policy enforced by prek's commit-msg hook.
// Types follow @commitlint/config-conventional; body lines may exceed 100
// chars since the repo history regularly wraps long URLs and references.
export default {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'body-max-line-length': [2, 'always', 200],
  },
};
