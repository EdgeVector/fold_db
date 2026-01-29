/**
 * @fileoverview Tests for Footer component
 * 
 * Tests the Footer component rendering and content display.
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import Footer from '../../components/Footer.jsx';

describe('Footer Component', () => {
  it('renders footer with correct terminal structure', () => {
    render(<Footer />);
    
    const footer = screen.getByRole('contentinfo');
    expect(footer).toBeInTheDocument();
    expect(footer).toHaveClass('bg-terminal-lighter', 'border-t', 'border-terminal', 'py-2');
  });

  it('displays fold_db branding', () => {
    render(<Footer />);
    
    expect(screen.getByText('fold_db')).toBeInTheDocument();
  });

  it('displays current year in copyright', () => {
    render(<Footer />);
    
    const currentYear = new Date().getFullYear();
    expect(screen.getByText(`© ${currentYear}`)).toBeInTheDocument();
  });

  it('displays version number', () => {
    render(<Footer />);
    
    expect(screen.getByText('node v1.0.0')).toBeInTheDocument();
  });

  it('has proper semantic structure', () => {
    render(<Footer />);
    
    const footer = screen.getByRole('contentinfo');
    const container = footer.querySelector('.max-w-7xl.mx-auto.px-6.text-center');
    expect(container).toBeInTheDocument();
  });

  it('uses terminal color classes', () => {
    render(<Footer />);
    
    const textElement = screen.getByText('fold_db');
    expect(textElement).toHaveClass('text-terminal-green');
  });

  it('renders without crashing', () => {
    expect(() => render(<Footer />)).not.toThrow();
  });
});
