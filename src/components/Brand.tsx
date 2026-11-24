/** The plain disc: an orange square with a charcoal circle. */
export function Disc({ size = 24 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 460 460" aria-hidden="true" className="disc">
      <rect width="460" height="460" fill="var(--accent-mark)" />
      <circle cx="230" cy="230" r="145" fill="var(--charcoal)" />
    </svg>
  );
}

/** The disc, then the product name. */
export function Lockup({ size = 24 }: { size?: number }) {
  return (
    <span className="lockup">
      <Disc size={size} />
      <span className="lockup-name">Claude Overview</span>
    </span>
  );
}
