# QueryGraph TypeScript

The TypeScript API is the Node.js equivalent of QueryGraph's Python API. It
uses the same Semantic Croissant, CDIF, OSI, ODRL, TypeDID, OpenLineage, and
Navigator contracts while keeping protected memory and policy authority in the
Rust service and TypeSec/Marciana stack.

```bash
npm install @querygraph/querygraph
```

```ts
import { CroissantDataset, Field, RecordSet, buildAgentCard } from "@querygraph/querygraph";

const dataset = new CroissantDataset({
  id: "https://example.test/coffee",
  name: "Coffee prices",
  description: "Weekly market observations",
  license: "https://creativecommons.org/publicdomain/zero/1.0/",
  creators: ["QueryGraph"],
  files: [],
  recordSets: [new RecordSet({
    id: "coffee-records",
    name: "Coffee records",
    fields: [new Field({ name: "price", dataType: "Float", description: "USD/lb" })],
  })],
  keywords: ["coffee", "market"],
});

console.log(dataset.toJsonLd());
console.log(buildAgentCard());
```

Build and test from this directory with `npm install`, `npm run build`, and
`npm test`.
