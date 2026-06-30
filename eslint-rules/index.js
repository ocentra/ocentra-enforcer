import noAppStringLiterals from './no-app-string-literals.js';
import noNakedDomainStringTypes from './no-naked-domain-string-types.js';
import noRuntimeStringTypes from './no-runtime-string-types.js';

export default {
  rules: {
    'no-app-string-literals': noAppStringLiterals,
    'no-naked-domain-string-types': noNakedDomainStringTypes,
    'no-runtime-string-types': noRuntimeStringTypes,
  },
};
