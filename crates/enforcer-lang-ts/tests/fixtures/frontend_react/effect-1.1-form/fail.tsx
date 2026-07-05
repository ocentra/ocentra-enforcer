import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { OrderSchema } from "./schema";

export function OrderForm() {
  const form = useForm({ resolver: zodResolver(OrderSchema) });
  return form;
}
