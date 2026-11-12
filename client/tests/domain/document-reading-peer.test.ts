import { afterAll, describe, expect, test } from "bun:test";
import { $fetch } from "ofetch";

import { TRANSLATION_BATCH, translated, translatorOf } from "bmo-ui/app/domain/translation";

import { documentReading, scriptOf, sectionsOf } from "../../app/domain/document-reading";

import type { Subprocess } from "bun";

// The reader's tab asked through bmo-ui's own translation module and the console's fetcher, of a stand-in
// translator killed while an ask is out and started again on the same port (docs/artifact.md, 「Where a person
// reads it」): the parts it did not answer stand in the original and are said to, and asking again once it is back
// asks for those parts alone and fills them.

const SCRIPTS = [scriptOf("en", "Latin")!, scriptOf("el", "Greek")!];
const BODY = ["Lead words.", "", "## First", "", "First words.", "", "## Second", "", "- one", "- two", "", "## Third", "", "Third words.", ""].join("\n");
const STAND_IN = new URL("../translator-stand-in.ts", import.meta.url).pathname;

// The stand-ins started, each taken down by the test or, at the latest, after it.
const started: Subprocess<"ignore", "pipe", "inherit">[] = [];

// One stand-in and what it has said so far, line by line.
interface StandIn {
  process: Subprocess<"ignore", "pipe", "inherit">;
  lines: string[];
  // Waits for the first line said that holds `words`, and answers it.
  said: (words: string) => Promise<string>;
}

function standIn(port: number, hold: number): StandIn {
  const process = Bun.spawn(["bun", STAND_IN, String(port), String(hold)], { stdout: "pipe", stderr: "inherit" });
  const lines: string[] = [];
  const waiting: { words: string; found: (line: string) => void }[] = [];

  started.push(process);

  void (async () => {
    let rest = "";

    for await (const chunk of process.stdout.pipeThrough(new TextDecoderStream())) {
      const parts = (rest + chunk).split("\n");

      rest = parts.pop()!;

      for (const line of parts) {
        lines.push(line);

        for (const one of waiting.filter((wait) => line.includes(wait.words))) {
          waiting.splice(waiting.indexOf(one), 1);
          one.found(line);
        }
      }
    }
  })();

  const said = (words: string) => {
    const line = lines.find((one) => one.includes(words));

    return line !== undefined ? Promise.resolve(line) : new Promise<string>((found) => waiting.push({ words, found }));
  };

  return { process, lines, said };
}

// The originals of every ask a stand-in took.
function asksOf(standing: StandIn): string[][] {
  return standing.lines.filter((line) => line.startsWith("ask ")).map((line) => JSON.parse(line.slice(4)) as string[]);
}

// Waits, a bounded number of turns of the event loop and no clock, for a condition the asks bring about.
async function until(condition: () => boolean): Promise<void> {
  for (let turn = 0; turn < 20_000 && !condition(); turn += 1) await new Promise((next) => setImmediate(next));

  expect(condition()).toBe(true);
}

afterAll(async () => {
  for (const process of started) process.kill("SIGKILL");
  await Promise.all(started.map((process) => process.exited));
});

describe("the reader's tab through a translator stopped and started again", () => {
  test("keeps what came, says the rest stands in the original, and fills it once asked again", async () => {
    // The first stand-in answers the first ask and holds the second.
    const first = standIn(0, 2);
    const port = Number((await first.said("listening ")).split(" ")[1]);

    // What the console gives a bundle's module: its app and runtime configuration, naming the translator, and
    // its fetcher.
    Object.assign(globalThis, {
      useAppConfig: () => ({ bmoUi: { relayBase: `http://127.0.0.1:${port}/relay` } }),
      useRuntimeConfig: () => ({ app: { baseURL: "/" }, public: { bmoUi: { translator: { service: "gate", path: "/translations", batch: 2 } } } }),
      $fetch
    });

    const reading = documentReading(() => ({ title: "Plain title", markdown: BODY }), {
      reader: "el",
      scripts: SCRIPTS,
      translate: translated,
      batch: translatorOf()?.batch ?? TRANSLATION_BATCH
    });
    const sections = sectionsOf(BODY);

    reading.tab.value = "reader";

    // The translator is killed while the second ask is out.
    await first.said("held");
    first.process.kill("SIGKILL");
    await first.process.exited;
    await until(() => reading.state.value !== "asking");

    expect(asksOf(first)).toEqual([
      ["Plain title", sections[0]!],
      [sections[1]!, sections[2]!]
    ]);
    expect(reading.state.value).toBe("partial");
    expect(reading.title.value).toBe("Plain title [el]");
    expect(reading.markdown.value).toBe(`Lead words. [el]\n\n${sections[1]}${sections[2]}${sections[3]}`);

    // Started again on the same address, it is asked for the parts that did not come and nothing else.
    const second = standIn(port, 0);

    await second.said("listening ");
    reading.again();
    await until(() => reading.state.value !== "asking");

    expect(asksOf(second).flat().sort()).toEqual([sections[1]!, sections[2]!, sections[3]!].sort());
    expect(reading.state.value).toBe("whole");
    expect(reading.markdown.value).toBe("Lead words. [el]\n\n## First\n\nFirst words. [el]\n\n## Second\n\n- one\n- two [el]\n\n## Third\n\nThird words. [el]\n");
  });
});
