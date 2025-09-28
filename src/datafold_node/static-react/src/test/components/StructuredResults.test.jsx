import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import StructuredResults from '../../components/StructuredResults.jsx'

const sample = {
  data: {
    H1: {
      R1: { a: 1, b: 'x' },
      R2: { c: true }
    },
    H2: {
      R1: { d: 4 }
    }
  }
}

describe('StructuredResults', () => {
  it('shows counts for hashes and ranges', () => {
    render(<StructuredResults results={sample} />)
    expect(screen.getByText(/Hashes:/)).toBeInTheDocument()
    expect(screen.getByText(/Ranges:/)).toBeInTheDocument()
  })

  it('expands hash and range to reveal fields', () => {
    render(<StructuredResults results={sample} pageSize={10} />)
    // Expand first hash
    const hashBtn = screen.getByRole('button', { name: /hash: H1/ })
    fireEvent.click(hashBtn)
    // Expand first range
    const rangeBtn = screen.getByRole('button', { name: /range: R1/ })
    fireEvent.click(rangeBtn)
    // See a field
    expect(screen.getByText('a')).toBeInTheDocument()
  })

  it('supports lazy show more for hashes', () => {
    const big = { data: {} }
    for (let i = 0; i < 120; i++) {
      big.data['H' + i] = { R1: { f: i } }
    }
    render(<StructuredResults results={big} pageSize={50} />)
    // Show more button visible
    const btn = screen.getByRole('button', { name: /Show more hashes/ })
    expect(btn).toBeInTheDocument()
  })
})


