export function formatDateTime(dateTime: string) {
  return new Date(dateTime).toLocaleString();
};