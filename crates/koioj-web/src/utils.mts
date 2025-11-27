export function formatDateTime(dateTime: string) {
  return new Date(dateTime).toLocaleString();
};

export function parseIntOrNull(value: string | string[] | null | undefined): number | null {
  if (value == null || Array.isArray(value)) return null;
  const parsed = parseInt(value, 10);
  return Number.isNaN(parsed) ? null : parsed;
}

export const SOURCE_REPO = `https://github.com/ParaN3xus/koioj/`

export const APP_NAME = 'KoiOJ'