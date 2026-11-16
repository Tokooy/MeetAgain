import { useEffect, useState, type ReactNode } from "react";
export function PanelSection({
  title,
  icon,
  children,
}: {
  title: string;
  icon: ReactNode;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(
    () => !window.matchMedia("(max-width: 850px)").matches,
  );
  useEffect(() => {
    const media = window.matchMedia("(max-width: 850px)");
    const update = () => setOpen(!media.matches);
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);
  return (
    <details
      className="mobile-section"
      open={open}
      onToggle={(e) => setOpen(e.currentTarget.open)}
    >
      <summary>
        {title}
        {icon}
      </summary>
      {children}
    </details>
  );
}
