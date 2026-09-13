export type Field = {
  key: string;
  label: string;
  vocabulary?: "relationships";
  type?:
    | "text"
    | "number"
    | "textarea"
    | "select"
    | "reference"
    | "checkbox"
    | "date"
    | "password";
  options?: string[];
  table?: string;
  required?: boolean;
  default?: string | number;
  wide?: boolean;
};
const f = (key: string, label: string, extras: Partial<Field> = {}): Field => ({
  key,
  label,
  ...extras,
});
const ref = (key: string, label: string, table: string, required = true) =>
  f(key, label, { type: "reference", table, required });
const select = (key: string, label: string, options: string[]) =>
  f(key, label, { type: "select", options, default: options[0] });
const note = f("notes", "Analyst notes", { type: "textarea", wide: true });
export const schemas: Record<string, Field[]> = {
  assessment_checks: [
    f("name", "Check name", { required: true, wide: true }),
    f("area", "Testing domain", { required: true, default: "Custom" }),
    f("owner", "Owner"),
    select("priority", "Priority", ["Normal", "Critical", "High", "Low"]),
    select("state", "State", [
      "Not started",
      "In progress",
      "Blocked",
      "Passed",
      "Failed",
      "Not applicable",
    ]),
    f("due_date", "Due date", { type: "date" }),
    ref("asset_id", "Target asset", "assets", false),
    f("objective", "Objective", { type: "textarea", wide: true }),
    f("result", "Result / reason", { type: "textarea", wide: true }),
    ref("evidence_id", "Supporting evidence", "evidence", false),
    ref("finding_id", "Related finding", "findings", false),
    f("reference_url", "Methodology reference", { wide: true }),
  ],
  assets: [
    f("name", "Display name", { required: true }),
    select("kind", "Asset type", [
      "Host",
      "Network",
      "Domain",
      "Web Application",
      "User",
    ]),
    f("ip", "IPv4 address"),
    f("hostname", "Hostname / FQDN"),
    f("os", "Operating system"),
    select("status", "Status", ["Active", "Inactive", "Unknown"]),
    select("access", "Access state", [
      "None",
      "User",
      "Administrator",
      "root",
      "Unknown",
    ]),
    f("direct", "Directly reachable", { type: "checkbox", default: 0 }),
    f("tags", "Tags · comma separated", { wide: true }),
    note,
  ],
  services: [
    ref("asset_id", "Host", "assets"),
    f("name", "Service name", { required: true }),
    f("port", "Port", { type: "number", required: true }),
    select("protocol", "Protocol", ["tcp", "udp"]),
    f("product", "Product"),
    f("version", "Version"),
    select("status", "Observed state", [
      "open",
      "closed",
      "filtered",
      "unknown",
    ]),
    f("tls", "TLS observed", { type: "checkbox", default: 0 }),
    f("url", "URL"),
    f("title", "Page title"),
    f("banner", "Banner", { type: "textarea", wide: true }),
    note,
  ],
  credentials: [
    f("username", "Username / identifier", { required: true }),
    f("context", "Domain / context"),
    select("kind", "Credential type", [
      "Password",
      "NTLM hash",
      "API token",
      "SSH private key reference",
      "Generic secret",
      "Other",
    ]),
    f("secret", "Secret", { type: "password", required: true }),
    ref("source_id", "Discovered on", "assets", false),
    f("source", "Source / provenance"),
    note,
  ],
  credential_tests: [
    ref("credential_id", "Credential", "credentials"),
    ref("asset_id", "Target host", "assets"),
    ref("service_id", "Service", "services"),
    select("result", "Authentication result", ["Valid", "Invalid", "Unknown"]),
    select("privilege", "Confirmed privilege", [
      "Unknown",
      "User",
      "Administrator",
      "root",
    ]),
    f("source", "Source / validation method", {
      required: true,
      default: "Manual analyst observation",
    }),
    ref("evidence_id", "Supporting evidence", "evidence", false),
  ],
  sessions: [
    f("name", "Session label", { required: true }),
    ref("asset_id", "Host", "assets"),
    f("username", "User", { required: true }),
    select("privilege", "Privilege", [
      "Unknown",
      "User",
      "Administrator",
      "root",
    ]),
    select("kind", "Session type", [
      "SSH",
      "WinRM",
      "RDP",
      "Shell",
      "SMB administrative access",
      "Other",
    ]),
    select("state", "State", ["Active", "Stale", "Closed", "Unknown"]),
    f("source", "Source"),
    ref("credential_id", "Associated credential", "credentials", false),
    f("pivot_capable", "Pivot capable", { type: "checkbox", default: 0 }),
    note,
  ],
  pivots: [
    f("name", "Pivot name", { required: true }),
    ref("session_id", "Source session", "sessions"),
    ref("asset_id", "Source host", "assets"),
    f("network", "Reachable IPv4 network", { required: true }),
    select("kind", "Pivot type", [
      "Ligolo",
      "SSH tunnel",
      "SOCKS",
      "Port forward",
      "Generic tunnel",
    ]),
    select("status", "Status", ["Active", "Inactive", "Unknown"]),
    note,
  ],
  findings: [
    f("name", "Finding title", { required: true, wide: true }),
    select("severity", "Severity", [
      "Info",
      "Low",
      "Medium",
      "High",
      "Critical",
    ]),
    select("status", "Status", [
      "Draft",
      "Confirmed",
      "Remediated",
      "Accepted",
      "False Positive",
    ]),
    ref("asset_id", "Affected asset", "assets"),
    ref("service_id", "Affected service", "services", false),
    f("description", "Description", { type: "textarea", wide: true }),
    f("impact", "Impact", { type: "textarea", wide: true }),
    f("reproduction", "Reproduction / validation", {
      type: "textarea",
      wide: true,
    }),
    f("remediation", "Remediation", { type: "textarea", wide: true }),
    f("refs", "References", { type: "textarea", wide: true }),
    f("cve", "CVE"),
    f("cwe", "CWE"),
    f("cvss", "CVSS"),
  ],
  evidence: [
    f("name", "Evidence title", { required: true, wide: true }),
    select("kind", "Evidence type", [
      "Text note",
      "Terminal output",
      "Command",
      "HTTP request",
      "HTTP response",
      "Screenshot",
      "File attachment",
    ]),
    ref("entity_id", "Linked entity", "all"),
    f("body", "Evidence content", { type: "textarea", wide: true }),
  ],
  relationships: [
    ref("source_id", "Source", "all"),
    ref("target_id", "Target", "all"),
    f("kind", "Relationship", {
      type: "select",
      required: true,
      vocabulary: "relationships",
    }),
    note,
  ],
  engagement: [
    f("name", "Engagement name", { required: true, wide: true }),
    f("description", "Description", { type: "textarea", wide: true }),
    select("kind", "Engagement type", [
      "Lab / CTF",
      "Internal Pentest",
      "External Pentest",
      "Web / API",
      "Other",
    ]),
    select("status", "Status", ["Active", "Paused", "Completed", "Archived"]),
    f("client", "Client / project reference"),
    f("start_date", "Start date", { type: "date" }),
  ],
};
export const singular: Record<string, string> = {
  assessment_checks: "assessment check",
  assets: "asset",
  services: "service",
  credentials: "credential",
  credential_tests: "authentication test",
  sessions: "session",
  pivots: "pivot",
  findings: "finding",
  evidence: "evidence",
  relationships: "relationship",
  engagement: "engagement",
};
