# HadoopCatalog to LakeCat migration cookbook

This cookbook proves metadata-pointer migration from Apache Iceberg's
filesystem-backed `HadoopCatalog` to LakeCat's standard Iceberg REST API. It is
an executable legacy-catalog path, not a claim about Hive Metastore or AWS Glue.

The fresh runner in `querygraph/catalog-bench` starts shared MinIO and a
source-built LakeCat, then runs stock Apache Spark 4.1.3 with Apache Iceberg
1.11.0. The source table is partitioned and non-empty, receives an additive
schema change, a second partition spec, multiple snapshots, and an `audit`
branch. Spark registers its current metadata file through LakeCat's standard
REST catalog and independently compares:

- schema and required/nullability representation;
- all partition specs and the current spec;
- all snapshots and the current snapshot;
- all refs, including `main` and `audit`;
- the exact metadata location, retained in evidence by digest; and
- exact source and destination data scans.

Run from the catalog-bench checkout:

```sh
QUERYGRAPH_ROOT="$HOME/src/querygraph" \
  docker/run-querygraph-hadoop-migration.sh hadoop_YYYYMMDDa
```

The runner refuses dirty migration source, reused output, reused Compose state,
or malformed run IDs. Its output contains no credentials or row values and it
deletes every run-owned container and volume. This workflow shares existing
metadata/data objects. It does not copy objects, orchestrate dual writers, or
prove Hive/Glue-specific permissions and locking.
