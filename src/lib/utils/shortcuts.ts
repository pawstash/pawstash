export function matchesShortcut(event: KeyboardEvent, shortcutStr: string): boolean {
  if (!shortcutStr || !shortcutStr.trim()) return false;

  const tokens = shortcutStr
    .split('+')
    .map((t) => t.trim().toLowerCase())
    .filter(Boolean);

  if (tokens.length === 0) return false;

  const expectsCtrl = tokens.includes('control') || tokens.includes('ctrl');
  const expectsAlt = tokens.includes('alt');
  const expectsShift = tokens.includes('shift');
  const expectsMeta = tokens.includes('super') || tokens.includes('meta') || tokens.includes('cmd');

  if (Boolean(event.ctrlKey) !== expectsCtrl) return false;
  if (Boolean(event.altKey) !== expectsAlt) return false;
  if (Boolean(event.shiftKey) !== expectsShift) return false;
  if (Boolean(event.metaKey) !== expectsMeta) return false;

  const primaryToken = tokens.find(
    (t) => !['control', 'ctrl', 'alt', 'shift', 'super', 'meta', 'cmd'].includes(t)
  );

  if (!primaryToken) return false;

  const evKey = (event.key || '').toLowerCase();
  const evCode = (event.code || '').toLowerCase();

  if (primaryToken === 'escape' || primaryToken === 'esc') {
    return evKey === 'escape' || evCode === 'escape';
  }

  if (primaryToken === 'space') {
    return evKey === ' ' || evKey === 'space' || evCode === 'space';
  }

  if (/^f\d{1,2}$/.test(primaryToken)) {
    return evKey === primaryToken || evCode === primaryToken;
  }

  if (primaryToken.length === 1 && primaryToken >= 'a' && primaryToken <= 'z') {
    return evKey === primaryToken || evCode === `key${primaryToken}`;
  }

  if (primaryToken.length === 1 && primaryToken >= '0' && primaryToken <= '9') {
    return evKey === primaryToken || evCode === `digit${primaryToken}` || evCode === `numpad${primaryToken}`;
  }

  return evKey === primaryToken || evCode === primaryToken;
}
