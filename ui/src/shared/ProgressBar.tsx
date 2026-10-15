export const percent = (done: number, total: number) =>
  total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 0

export const ProgressBar = ({ done, total }: { done: number; total: number }) => (
  <span
    role="progressbar"
    aria-valuemin={0}
    aria-valuemax={100}
    aria-valuenow={percent(done, total)}
    className="h-1.5 w-40 overflow-hidden rounded-full bg-twilight"
  >
    <span
      className="block h-full bg-ember transition-[width]"
      style={{ width: `${percent(done, total)}%` }}
    />
  </span>
)
