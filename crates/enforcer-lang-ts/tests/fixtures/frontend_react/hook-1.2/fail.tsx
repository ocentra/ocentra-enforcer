import { useEffect } from "react";

export function FocusTrap({ active }: { active: boolean }) {
  useEffect(() => {
    if (active) {
      document.getElementById("modal")?.focus();
    }
  }, [active]);

  return null;
}
