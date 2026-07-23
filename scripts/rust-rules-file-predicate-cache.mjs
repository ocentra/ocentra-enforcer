function createFilePredicate(cacheFilePredicates, predicateStats) {
  return (cachedValue, compute) => {
    if (cacheFilePredicates) return cachedValue;
    if (predicateStats) {
      predicateStats.uncachedEvaluations =
        (predicateStats.uncachedEvaluations ?? 0) + 1;
    }
    return compute();
  };
}

export { createFilePredicate };
