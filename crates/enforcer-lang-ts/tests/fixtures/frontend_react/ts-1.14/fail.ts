import { User } from "@/types/user";

export function describe(user: User): string {
  return `${user.id}`;
}
