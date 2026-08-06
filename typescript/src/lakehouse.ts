export interface TableSpec { name: string; location: string; format: string }
export function exampleQueries(scope = "global_temp"): string[] { return [`SELECT COUNT(*) FROM ${scope}.government_finance__countydata`, `SELECT * FROM ${scope}.openlineage_events LIMIT 10`]; }
export function findLatestParquetDir(warehouse: string, table: string): string { return `${warehouse}/${table}`; }
