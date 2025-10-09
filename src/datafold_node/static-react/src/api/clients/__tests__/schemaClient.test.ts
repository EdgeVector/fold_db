import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { UnifiedSchemaClient } from '../schemaClient';
import { ApiClient } from '../../core/client';

describe('UnifiedSchemaClient', () => {
  let client: UnifiedSchemaClient;
  let mockApi: Pick<ApiClient, 'get'>;
  let consoleWarnSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    mockApi = {
      // @ts-expect-error - we only mock the methods we use
      get: vi.fn()
    };
    // @ts-expect-error - pass partial mock
    client = new UnifiedSchemaClient(mockApi);
    consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleWarnSpy.mockRestore();
  });

  describe('getSchemas normalization', () => {
    it('handles direct array response', async () => {
      (mockApi.get as any).mockResolvedValue({ success: true, data: [{ name: 'A' }, { name: 'B' }] });
      const res = await client.getSchemas();
      expect(res.success).toBe(true);
      expect(Array.isArray(res.data)).toBe(true);
      expect(res.data?.map(s => (s as any).name)).toEqual(['A', 'B']);
    });

    it('normalizes object map { name: Schema } to array', async () => {
      (mockApi.get as any).mockResolvedValue({ success: true, data: { A: { name: 'A' }, B: { name: 'B' } } });
      const res = await client.getSchemas();
      expect(res.success).toBe(true);
      expect(Array.isArray(res.data)).toBe(true);
      expect(res.data?.map(s => (s as any).name).sort()).toEqual(['A', 'B']);
    });

    it('returns empty array on unexpected shape', async () => {
      (mockApi.get as any).mockResolvedValue({ success: true, data: 'weird' });
      const res = await client.getSchemas();
      expect(res.success).toBe(true);
      expect(res.data).toEqual([]);
    });
  });

  describe('getAllSchemasWithState', () => {
    it('normalizes state values to lowercase map entries', async () => {
      (mockApi.get as any).mockResolvedValue({
        success: true,
        data: [{ name: 'BlogPost', state: 'Approved' }]
      });

      const res = await client.getAllSchemasWithState();
      expect(res.success).toBe(true);
      expect(res.data).toEqual({ BlogPost: 'approved' });
      expect(res.status).toBe(200);
      expect(consoleWarnSpy).not.toHaveBeenCalled();
    });

    it('falls back to available when state is missing', async () => {
      (mockApi.get as any).mockResolvedValue({
        success: true,
        data: [{ name: 'BlogPost' }]
      });

      const res = await client.getAllSchemasWithState();
      expect(res.success).toBe(true);
      expect(res.data).toEqual({ BlogPost: 'available' });
      expect(consoleWarnSpy).toHaveBeenCalled();
    });

    it('propagates failure from getSchemas', async () => {
      (mockApi.get as any).mockResolvedValue({
        success: false,
        status: 500,
        error: 'boom',
        data: []
      });

      const res = await client.getAllSchemasWithState();
      expect(res.success).toBe(false);
      expect(res.data).toEqual({});
      expect(res.status).toBe(500);
      expect(consoleWarnSpy).not.toHaveBeenCalled();
    });
  });
});

