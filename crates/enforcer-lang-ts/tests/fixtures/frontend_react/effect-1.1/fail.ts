import { z } from "zod";

export const OrderSchema = z.object({
  id: z.string(),
  total: z.number(),
});
