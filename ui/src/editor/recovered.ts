/** When recovered changes were written, as the user's locale shows dates. */
export function savedWhen(savedAt: number): string {
  return new Date(savedAt).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}
