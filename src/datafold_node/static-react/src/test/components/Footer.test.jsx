/**
 * @fileoverview Tests for Footer component
 * 
 * Tests the Footer component rendering and content display.
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import Footer from '../../components/Footer.jsx';

describe('Footer Component', () => {
  it('renders footer with correct structure', () => {
    render(<Footer />);
    
    const footer = screen.getByRole('contentinfo');
    expect(footer).toBeInTheDocument();
    expect(footer).toHaveClass('bg-white', 'border-t', 'border-gray-200', 'py-6', 'mt-auto');
  });

  it('displays current year in copyright text', () => {
    render(<Footer />);
    
    const currentYear = new Date().getFullYear();
    const copyrightText = screen.getByText(`DataFold Node © ${currentYear}`);
    expect(copyrightText).toBeInTheDocument();
    expect(copyrightText).toHaveClass('text-gray-600', 'text-sm');
  });

  it('has proper semantic structure', () => {
    render(<Footer />);
    
    const footer = screen.getByRole('contentinfo');
    const container = footer.querySelector('.max-w-7xl.mx-auto.px-6.text-center');
    expect(container).toBeInTheDocument();
  });

  it('renders without crashing', () => {
    expect(() => render(<Footer />)).not.toThrow();
  });
});

