import { useForm } from "react-hook-form";
import { effectResolver } from "@/lib/effect-resolver";
import { OrderSchema } from "./schema";

export function OrderForm() {
  const form = useForm({ resolver: effectResolver(OrderSchema) });
  return form;
}
