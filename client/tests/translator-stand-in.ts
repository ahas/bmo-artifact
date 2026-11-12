// A stand-in for the console's relay of the translator, run as a process of its own so a test can stop it while
// an ask is out and start it again: `bun tests/translator-stand-in.ts <port> <ask to hold>`. It reads every
// original into the reader's language, keeping its shape, as `<original> [<language>]`, and holds the ask whose
// number is given, answering it never. It says on its standard output the port it answers on, each ask's
// originals as one JSON line, and when it holds an ask.

const [port, hold] = process.argv.slice(2).map(Number);

// The test's document stands on another origin than the stand-in, so its fetcher asks first.
const OPEN = { "access-control-allow-origin": "*", "access-control-allow-headers": "*", "access-control-allow-methods": "POST" };

let asked = 0;

const server = Bun.serve({
  port: port ?? 0,
  hostname: "127.0.0.1",
  async fetch(request) {
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: OPEN });

    const { language, originals } = (await request.json()) as { language: string; originals: string[] };

    asked += 1;
    console.log(`ask ${JSON.stringify(originals)}`);

    if (asked === hold) {
      console.log("held");
      await new Promise(() => {});
    }

    return Response.json({ readings: originals.map((text) => ({ text: `${text.trim()} [${language}]`, whole: true })) }, { headers: OPEN });
  }
});

console.log(`listening ${server.port}`);
