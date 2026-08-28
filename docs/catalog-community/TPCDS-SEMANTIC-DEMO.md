# TPC-DS semantic supply-chain demo

Run from a clean `querygraph/catalog-bench` checkout:

```bash
docker/run-querygraph-tpcds-fixture.sh tpcds_<unique-id>
```

The command fetches the checksum-pinned Apache Ossie model, creates five
Iceberg tables through stock Spark/Iceberg REST, hashes their realized schemas
and snapshots, installs a governed LakeCat policy, CAS-publishes the exact
model artifact, and drains publication replay into graph and OpenLineage
receipts. It then executes five representative semantic answers and binds them
to physical, model, artifact, policy, plan, graph, and lineage hashes.

The final adversarial step mutates physical, model, policy, graph, lineage, and
artifact bases independently. Every mutation must invalidate the saved proof.
The runner refuses an existing output directory, uses run-owned state, and
removes all labeled containers and volumes. The reviewed reference result is
`catalog-bench/results/source/semantic/tpcds_0828g/`.

This demonstrates a correctness and provenance supply chain. It does not claim
TPC-DS performance, full TPC-DS scale, distributed exactly-once projection, or
lossless interchange with every Ossie converter.
