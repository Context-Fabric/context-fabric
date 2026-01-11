/**
 * Helper functions for safely rendering documentation content
 */

/**
 * Safely convert a value to a renderable string
 */
export function toRenderableString(value: unknown): string {
  if (value === null || value === undefined) {
    return "";
  }
  if (typeof value === "string") {
    return value;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  // For objects and arrays, don't render them directly
  return "";
}

/**
 * Check if a value is a non-empty renderable string
 */
export function isRenderableString(value: unknown): value is string {
  return typeof value === "string" && value.length > 0;
}
