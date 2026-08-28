# Apache Ossie upstream contract

QueryGraph consumes, but does not fork, Apache Ossie's evolving semantic-model
contract. `ossie/upstream.json` pins commit
`1d9ebcea2932d3381c0840cc8304f0850d366509` and the SHA-256 of the upstream
Draft 2020-12 schema, validator, and TPC-DS example.

Fetch and verify the accepted bytes into a disposable directory:

```sh
python scripts/fetch-ossie.py fetch target/ossie-upstream
python scripts/fetch-ossie.py verify target/ossie-upstream
uv run --with 'jsonschema>=4.26' --with 'pyyaml>=6.0.3' \
  --with 'sqlglot>=30.12' target/ossie-upstream/validation/validate.py \
  target/ossie-upstream/examples/tpcds_semantic_model.yaml
```

Fetched files are build inputs, not maintained copies. A new upstream revision
requires an explicit manifest update and renewed validation/loss evidence.
