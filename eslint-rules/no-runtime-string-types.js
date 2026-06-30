export default {
  meta: {
    type: 'problem',
    docs: {
      description: 'Disallow raw string types in runtime app source files.',
    },
    messages: {
      runtimeStringType:
        'Runtime app source cannot type values as raw string. Use a branded domain type or keep the boundary as unknown until parsed.',
    },
    schema: [],
  },
  create(context) {
    return {
      TSStringKeyword(node) {
        context.report({ node, messageId: 'runtimeStringType' });
      },
    };
  },
};
