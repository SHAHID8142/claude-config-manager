/** V2-21: how a stored key is shown once it is saved — never the key itself. */
export function maskKey(last4: string): string {
  return last4 === "" ? "••••" : `•••• ${last4}`;
}
