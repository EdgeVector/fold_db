import { useRef, useEffect } from 'react'

/**
 * Animated FoldDB logo rendered on a <canvas>.
 * @param {{ size?: number, className?: string }} props
 */
function AnimatedLogo({ size = 84, className = '' }) {
  const canvasRef = useRef(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const dpr = window.devicePixelRatio || 1
    canvas.width = size * dpr
    canvas.height = size * dpr
    canvas.style.width = size + 'px'
    canvas.style.height = size + 'px'
    ctx.scale(dpr, dpr)

    const S = size
    const CX = S / 2, CY = S / 2

    const lerp = (a, b, t) => a + (b - a) * t
    const ease = (t) => t < 0.5 ? 2*t*t : 1 - Math.pow(-2*t+2, 2)/2

    const COLORS = ['#E53935', '#FF9800', '#FDD835']
    const LINE_W = Math.max(1, S * 0.0075)

    const sc = S / 400
    const RECT_W   = 60 * sc
    const RECT_H   = 30 * sc
    const RECT_R   = 14 * sc
    const CIRCLE_R = 60 * sc
    const SCALE_Y  = 0.5

    const OFFSETS_RECT   = [-36 * sc, 0, 36 * sc]
    const OFFSETS_CIRCLE = [-34 * sc, 0, 34 * sc]

    const SPIN_DUR  = 0.14
    const HOLD_RECT = 0.06
    const HOLD_CIRC = 0.08

    const FWD_START = [
      HOLD_RECT,
      HOLD_RECT + SPIN_DUR,
      HOLD_RECT + SPIN_DUR * 2
    ]
    const REV_BEGIN = HOLD_RECT + SPIN_DUR * 3 + HOLD_CIRC
    const REV_START = [
      REV_BEGIN + SPIN_DUR * 2,
      REV_BEGIN + SPIN_DUR,
      REV_BEGIN
    ]

    function getShapeState(p, i) {
      const fs = FWD_START[i], fe = fs + SPIN_DUR
      const rs = REV_START[i], re = rs + SPIN_DUR
      let t, rot
      if (p < fs)      { t = 0; rot = 0 }
      else if (p < fe) { const raw = ease((p - fs) / SPIN_DUR); t = raw; rot = raw * Math.PI * 2 }
      else if (p < rs) { t = 1; rot = 0 }
      else if (p < re) { const raw = ease((p - rs) / SPIN_DUR); t = 1 - raw; rot = raw * Math.PI * 2 }
      else             { t = 0; rot = 0 }
      return { t, rot }
    }

    function drawShape(cx, cy, hw, hh, r, color, sy, rot, t) {
      const clampR = Math.min(r, hw, hh)
      const L = cx - hw, R = cx + hw, T = cy - hh, B = cy + hh
      ctx.save()
      ctx.translate(cx, cy)
      ctx.rotate(rot)
      ctx.scale(1, sy)
      ctx.translate(-cx, -cy)
      ctx.beginPath()
      ctx.moveTo(L + clampR, T)
      ctx.lineTo(R - clampR, T)
      ctx.arcTo(R, T, R, T + clampR, clampR)
      ctx.lineTo(R, B - clampR)
      ctx.arcTo(R, B, R - clampR, B, clampR)
      ctx.lineTo(L + clampR, B)
      ctx.arcTo(L, B, L, B - clampR, clampR)
      ctx.lineTo(L, T + clampR)
      ctx.arcTo(L, T, L + clampR, T, clampR)
      ctx.closePath()
      if (t > 0) {
        ctx.globalAlpha = t
        ctx.fillStyle = color
        ctx.fill()
      }
      ctx.globalAlpha = 1
      ctx.strokeStyle = color
      ctx.lineWidth = LINE_W
      ctx.stroke()
      ctx.restore()
    }

    function draw(p) {
      ctx.clearRect(0, 0, S, S)
      for (let i = 2; i >= 0; i--) {
        const { t, rot } = getShapeState(p, i)
        const hw = lerp(RECT_W, CIRCLE_R, t)
        const hh = lerp(RECT_H, CIRCLE_R, t)
        const r  = lerp(RECT_R, CIRCLE_R, t)
        const sy = lerp(1, SCALE_Y, t)
        const cy = CY + lerp(OFFSETS_RECT[i], OFFSETS_CIRCLE[i], t)
        drawShape(CX, cy, hw, hh, r, COLORS[i], sy, rot, t)
      }
    }

    const DUR = 6000
    let t0 = null
    let rafId
    function tick(ts) {
      if (!t0) t0 = ts
      draw(((ts - t0) % DUR) / DUR)
      rafId = window.requestAnimationFrame(tick)
    }
    rafId = window.requestAnimationFrame(tick)

    return () => window.cancelAnimationFrame(rafId)
  }, [size])

  return <canvas ref={canvasRef} className={className} style={{ display: 'block' }} />
}

export default AnimatedLogo
