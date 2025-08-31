/**
 * @fileoverview Tests for LogSidebar component
 * 
 * Tests the LogSidebar component including API interactions, log streaming,
 * and user interactions.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import LogSidebar from '../../components/LogSidebar.jsx';
import { systemClient } from '../../api/clients/systemClient';

// Mock the systemClient
vi.mock('../../api/clients/systemClient', () => ({
  systemClient: {
    getLogs: vi.fn(),
    createLogStream: vi.fn()
  }
}));

// Mock clipboard API
const mockWriteText = vi.fn();
Object.assign(navigator, {
  clipboard: {
    writeText: mockWriteText
  }
});

describe('LogSidebar Component', () => {
  const mockLogs = [
    '2024-01-01 10:00:00 INFO: Server started',
    '2024-01-01 10:00:01 DEBUG: Database connected',
    '2024-01-01 10:00:02 ERROR: Failed to load schema'
  ];

  const mockEventSource = {
    close: vi.fn()
  };

  beforeEach(() => {
    vi.clearAllMocks();
    
    // Mock successful getLogs response
    systemClient.getLogs.mockResolvedValue({
      success: true,
      data: { logs: mockLogs }
    });

    // Mock createLogStream to return a mock EventSource
    systemClient.createLogStream.mockReturnValue(mockEventSource);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders log sidebar with correct structure', () => {
    render(<LogSidebar />);
    
    expect(screen.getByText('Logs')).toBeInTheDocument();
    expect(screen.getByText('Copy')).toBeInTheDocument();
  });

  it('loads initial logs on mount', async () => {
    render(<LogSidebar />);
    
    await waitFor(() => {
      expect(systemClient.getLogs).toHaveBeenCalledTimes(1);
    });

    // Check that logs are displayed
    await waitFor(() => {
      mockLogs.forEach(log => {
        expect(screen.getByText(log)).toBeInTheDocument();
      });
    });
  });

  it('handles empty logs response', async () => {
    systemClient.getLogs.mockResolvedValue({
      success: true,
      data: { logs: [] }
    });

    render(<LogSidebar />);
    
    await waitFor(() => {
      expect(systemClient.getLogs).toHaveBeenCalledTimes(1);
    });

    // Should not crash with empty logs
    expect(screen.getByText('Logs')).toBeInTheDocument();
  });

  it('handles failed logs response', async () => {
    systemClient.getLogs.mockRejectedValue(new Error('API Error'));

    render(<LogSidebar />);
    
    await waitFor(() => {
      expect(systemClient.getLogs).toHaveBeenCalledTimes(1);
    });

    // Should not crash with failed API call
    expect(screen.getByText('Logs')).toBeInTheDocument();
  });

  it('sets up log streaming on mount', () => {
    render(<LogSidebar />);
    
    expect(systemClient.createLogStream).toHaveBeenCalledTimes(1);
    expect(systemClient.createLogStream).toHaveBeenCalledWith(
      expect.any(Function), // onMessage callback
      expect.any(Function)  // onError callback
    );
  });

  it('closes log stream on unmount', () => {
    const { unmount } = render(<LogSidebar />);
    
    unmount();
    
    expect(mockEventSource.close).toHaveBeenCalledTimes(1);
  });

  it('handles copy button click', async () => {
    const user = userEvent.setup();
    mockWriteText.mockResolvedValue();

    render(<LogSidebar />);
    
    // Wait for logs to load
    await waitFor(() => {
      expect(systemClient.getLogs).toHaveBeenCalledTimes(1);
    });

    const copyButton = screen.getByText('Copy');
    await user.click(copyButton);

    expect(mockWriteText).toHaveBeenCalledWith(
      mockLogs.join('\n')
    );
  });

  it('handles copy button click failure gracefully', async () => {
    const user = userEvent.setup();
    mockWriteText.mockRejectedValue(new Error('Copy failed'));

    render(<LogSidebar />);
    
    // Wait for logs to load
    await waitFor(() => {
      expect(systemClient.getLogs).toHaveBeenCalledTimes(1);
    });

    const copyButton = screen.getByText('Copy');
    
    // Should not throw error
    await expect(user.click(copyButton)).resolves.not.toThrow();
  });

  it('has correct CSS classes and structure', () => {
    render(<LogSidebar />);
    
    const sidebar = screen.getByText('Logs').closest('aside');
    expect(sidebar).toHaveClass('w-80', 'h-screen', 'bg-gray-900', 'text-white', 'p-4', 'overflow-y-auto');
    
    const copyButton = screen.getByText('Copy');
    expect(copyButton).toHaveClass('text-xs', 'text-blue-300', 'hover:underline');
  });

  it('renders without crashing', () => {
    expect(() => render(<LogSidebar />)).not.toThrow();
  });
});
