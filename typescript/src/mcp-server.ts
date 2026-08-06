import { Action } from "./odrl.js";
import { OdrlRightsLayer } from "./odrl-rights.js";
export function parseAction(value: string): Action | string { return Object.values(Action).includes(value as Action) ? value as Action : value; }
export function demoRightsLayer(): OdrlRightsLayer { return OdrlRightsLayer.demo(); }
export function loadRightsLayer(value: { policies: { id: string; permissions: { action: string; resource: string; subjects?: string[] }[] }[] }): OdrlRightsLayer { return new OdrlRightsLayer(value.policies); }
export function createServer(rights = OdrlRightsLayer.demo()): { tools: Record<string, (input: Record<string, unknown>) => unknown> } { return { tools: { check_access: (input) => rights.check(String(input.principal), String(input.action), String(input.resource)).toJson(), verify_envelope: (input) => Boolean(input.signature), health: () => ({ status: "ok" }) } }; }
export async function serve(server = createServer()): Promise<typeof server> { return server; }
