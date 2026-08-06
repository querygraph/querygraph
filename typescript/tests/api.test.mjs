import assert from "node:assert/strict";
import test from "node:test";
import {
  AiNavigator,
  CroissantDataset,
  Field,
  OsiDocument,
  RecordSet,
  TypeDidAgent,
  b58decode,
  b58encode,
  buildAgentCard,
  runIdFor,
} from "../dist/index.js";

test("base58 round-trips binary values", () => {
  const value = Uint8Array.from([0, 1, 2, 254, 255]);
  assert.deepEqual(Array.from(b58decode(b58encode(value))), Array.from(value));
});

test("TypeDID envelopes sign and verify their canonical payload", () => {
  const sender = TypeDidAgent.new("sender", "fixture-sender");
  const recipient = TypeDidAgent.new("recipient", "fixture-recipient");
  const envelope = sender.request(recipient, "read", "dataset:coffee", { price: 4.2 });
  assert.equal(envelope.isSigned(), true);
  assert.equal(envelope.verifySignature(), true);
  assert.equal(envelope.verifyPayload(), true);
});

test("Croissant, OSI, and Navigator projections compose", () => {
  const dataset = new CroissantDataset({
    id: "https://example.test/coffee",
    name: "Coffee prices",
    description: "Weekly prices",
    license: "CC0",
    creators: ["QueryGraph"],
    files: [],
    recordSets: [new RecordSet({ id: "records", name: "Records", fields: [new Field({ name: "price", dataType: "Float", description: "USD/lb", semanticTypeValue: "schema:price" })] })],
    keywords: ["coffee"],
  });
  const osi = OsiDocument.fromCroissant(dataset);
  assert.equal(osi.semanticModel.datasets[0]?.fields[0]?.name, "price");
  const output = new AiNavigator().build({ datasetName: dataset.name, description: dataset.description, landingPage: dataset.id, dataUrl: "https://example.test/coffee.csv" });
  assert.equal(output.croissant["@type"], "cr:Dataset");
  assert.equal(output.osi.semanticModel.name, "coffee_prices_semantic_model");
});

test("agent card and lineage IDs are deterministic", () => {
  assert.equal(buildAgentCard().name, "QueryGraph Navigator");
  assert.equal(runIdFor("fixture"), runIdFor("fixture"));
});
