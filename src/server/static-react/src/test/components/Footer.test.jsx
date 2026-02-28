/**
 * @fileoverview Tests for Footer component
 *
 * Tests the Footer component rendering and content display.
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import Footer from '../../components/Footer.jsx';

describe('Footer Component', () => {
  it('renders footer with structure', () => {
    render(<Footer />);

    const footer = screen.getByRole('contentinfo');
    expect(footer).toBeInTheDocument();
    expect(footer).toHaveClass('bg-surface', 'border-t', 'flex-shrink-0');
  });

  it('displays FoldDB branding', () => {
    render(<Footer />);

    expect(screen.getByText(/FoldDB/i)).toBeInTheDocument();
  });

  it('displays version number', () => {
    render(<Footer />);

    expect(screen.getByText(/v\d+\.\d+\.\d+/i)).toBeInTheDocument();
  });

  it('displays Local Mode indicator', () => {
    render(<Footer />);

    expect(screen.getByText('Local Mode')).toBeInTheDocument();
  });

  it('has proper layout structure', () => {
    render(<Footer />);

    const footer = screen.getByRole('contentinfo');
    const container = footer.firstChild;
    expect(container).toHaveClass('flex', 'items-center', 'justify-between');
  });

  it('renders without crashing', () => {
    expect(() => render(<Footer />)).not.toThrow();
  });
});
