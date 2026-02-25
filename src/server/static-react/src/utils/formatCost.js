/** Format a dollar amount for display — shows "Free (local)" when zero */
export const fmtCost = (v) => Number(v) === 0 ? 'Free (local)' : `$${Number(v).toFixed(2)}`
