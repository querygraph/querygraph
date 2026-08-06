export interface TableSpec { name: string; location: string; format: string }
export function loadTableSpecs(manifest: Record<string, unknown>): TableSpec[] { return ((manifest.tables ?? []) as Record<string, unknown>[]).map((table) => ({ name: String(table.name), location: String(table.location), format: String(table.format ?? "parquet") })); }
export function registerLakehouse(manifest: Record<string, unknown>, warehouse: string): { tables: TableSpec[]; warehouse: string } { return { tables: loadTableSpecs(manifest), warehouse }; }
export function registerAudit(warehouse: string): { warehouse: string; table: string } { return { warehouse, table: "openlineage_events" }; }
export function exampleQueries(scope = "global_temp"): string[] { return [`SELECT COUNT(*) FROM ${scope}.government_finance__countydata`, `SELECT * FROM ${scope}.openlineage_events LIMIT 10`]; }
export function findLatestParquetDir(warehouse: string, table: string): string { return `${warehouse}/${table}`; }
