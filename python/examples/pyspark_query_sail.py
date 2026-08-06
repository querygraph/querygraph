#!/usr/bin/env python3
"""Query the locally registered QueryGraph Sail warehouse with PySpark.

Start Sail first:

    sail spark server --port 50051

Then register tables:

    querygraph lakehouse-register --warehouse ../spark-warehouse \
      --manifest ../.querygraph/lakehouse/manifest/load-report.json
"""
from __future__ import annotations

from querygraph.lakehouse import example_queries, spark_session


def main() -> None:
    spark = spark_session("sc://127.0.0.1:50051")
    for sql in example_queries("global_temp"):
        print(f"\n-- {sql}")
        spark.sql(sql).show(truncate=False)


if __name__ == "__main__":
    main()
