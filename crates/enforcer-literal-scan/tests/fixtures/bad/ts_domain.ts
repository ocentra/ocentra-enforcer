export function run(status: string): void {
  if (status === "enabled") console.log("user.created");
  fetch("/api/users");
}
