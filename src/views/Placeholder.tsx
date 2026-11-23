export function Placeholder({ title, milestone, detail }: { title: string; milestone: number; detail: string }) {
  return (
    <section className="card placeholder">
      <h2>{title}</h2>
      <p className="muted">{detail}</p>
      <span className="badge">Milestone {milestone}</span>
    </section>
  );
}
