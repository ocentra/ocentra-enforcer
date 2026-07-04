export function activate(shared: SharedState): SharedState {
  return { ...shared, status: "active" };
}
