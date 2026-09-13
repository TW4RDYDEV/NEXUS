import { useEffect, useRef, type ReactNode } from "react";
import { X, ArrowUpRight, ChevronRight, LoaderCircle } from "lucide-react";
export function Brand({ large = false }: { large?: boolean }) {
  return (
    <div className={`brand ${large ? "brand-large" : ""}`}>
      <img src="/nexus.svg" alt="" />
      <div>
        <strong>NEXUS</strong>
        {large && <span>by TWARDY.exe</span>}
      </div>
    </div>
  );
}
export function Badge({
  children,
  tone,
}: {
  children: ReactNode;
  tone?: string;
}) {
  const value = String(children).toLowerCase();
  const type =
    tone ||
    (/valid|active|reachable|complete|confirmed|root|administrator|open|new/.test(
      value,
    ) &&
    !value.includes("invalid") &&
    !value.includes("inactive")
      ? "good"
      : /critical|high|invalid|blocked|failed|removed/.test(value)
        ? "bad"
        : /unknown|untested|partial|medium|changed|stale|draft/.test(value)
          ? "warn"
          : "neutral");
  return <span className={`badge ${type}`}>{children}</span>;
}
export function Empty({
  title,
  detail,
  action,
}: {
  title: string;
  detail: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty">
      <div className="empty-symbol">
        <ArrowUpRight size={21} />
      </div>
      <h3>{title}</h3>
      <p>{detail}</p>
      {action}
    </div>
  );
}
export function Modal({
  title,
  subtitle,
  children,
  onClose,
  wide = false,
}: {
  title: string;
  subtitle?: string;
  children: ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const d = ref.current;
    d?.showModal();
    return () => d?.close();
  }, []);
  return (
    <dialog
      ref={ref}
      className={wide ? "modal wide" : "modal"}
      onCancel={onClose}
      onClick={(e) => {
        if (e.target === ref.current) onClose();
      }}
      aria-label={title}
    >
      <header>
        <div>
          <h2>{title}</h2>
          {subtitle && <p>{subtitle}</p>}
        </div>
        <button
          className="icon-button"
          aria-label="Close dialog"
          onClick={onClose}
        >
          <X size={18} />
        </button>
      </header>
      {children}
    </dialog>
  );
}
export function PageTitle({
  eyebrow,
  title,
  description,
  actions,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="page-title">
      <div>
        {eyebrow && <div className="eyebrow">{eyebrow}</div>}
        <h1>{title}</h1>
        {description && <p>{description}</p>}
      </div>
      <div className="actions">{actions}</div>
    </div>
  );
}
export function Loading({ label = "Loading workspace" }: { label?: string }) {
  return (
    <div className="loading">
      <LoaderCircle className="spin" size={18} />
      {label}
    </div>
  );
}
export function InlineLink({
  children,
  onClick,
}: {
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button className="inline-link" onClick={onClick}>
      {children}
      <ChevronRight size={13} />
    </button>
  );
}
