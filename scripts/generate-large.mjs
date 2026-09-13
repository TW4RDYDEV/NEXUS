import fs from "node:fs";
import path from "node:path";
const count = Number(process.argv[2] || 10000);
if (!Number.isInteger(count) || count < 1 || count > 50000)
  throw Error("Choose 1–50000 hosts");
const destination = path.resolve("fixtures/large_generated_lab");
fs.mkdirSync(destination, { recursive: true });
const stream = fs.createWriteStream(path.join(destination, "nmap.xml"));
stream.write('<?xml version="1.0"?><nmaprun scanner="nmap">');
for (let i = 1; i <= count; i++) {
  const ip = `10.${Math.floor(i / 65536)}.${Math.floor(i / 256) % 256}.${i % 256}`;
  stream.write(
    `<host><address addr="${ip}" addrtype="ipv4"/><hostnames><hostname name="lab-${String(i).padStart(5, "0")}.example.test"/></hostnames><ports>${[
      [22, "ssh"],
      [80, "http"],
      [443, "https"],
      [445, "microsoft-ds"],
      [8080, "http"],
    ]
      .map(
        ([port, name]) =>
          `<port protocol="tcp" portid="${port}"><state state="open"/><service name="${name}" product="Fictional lab service" version="1.0"/></port>`,
      )
      .join("")}</ports></host>`,
  );
}
stream.end("</nmaprun>");
await new Promise((resolve, reject) => {
  stream.on("finish", resolve);
  stream.on("error", reject);
});
console.info(
  `Generated ${count} fictional hosts and ${count * 5} services at ${destination}`,
);
