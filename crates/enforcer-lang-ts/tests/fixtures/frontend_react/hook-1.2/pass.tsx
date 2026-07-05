import { useEffect } from "react";

export function FocusTrap({ active }: { active: boolean }) {
  // why: browser focus is imperative-only; there is no declarative prop
  // for "focus this element on mount", so it must live in an effect.
  useEffect(() => {
    if (active) {
      document.getElementById("modal")?.focus();
    }
  }, [active]);

  return null;
}
