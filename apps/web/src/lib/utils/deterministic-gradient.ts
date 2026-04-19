export function deterministicPubkeyGradient(pubkey: string): string {
  const hex = pubkey.slice(0, 6).padEnd(6, '0');
  const r = Number.parseInt(hex.slice(0, 2), 16);
  const g = Number.parseInt(hex.slice(2, 4), 16);
  const b = Number.parseInt(hex.slice(4, 6), 16);
  const hue = (r * 3 + g * 5 + b * 7) % 360;

  return `linear-gradient(135deg, #${hex}, hsl(${hue} 55% 42%))`;
}
