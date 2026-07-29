import { logger } from "./logging/logger";

export function trace(message: string): void {
  logger.info(message);
}
