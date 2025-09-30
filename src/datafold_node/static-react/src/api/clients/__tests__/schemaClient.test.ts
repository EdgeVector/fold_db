import { describe, it, expect, vi, beforeEach } from 'vitest';
import { UnifiedSchemaClient } from '../schemaClient';
import { ApiClient } from '../../core/client';

describe('UnifiedSchemaClient.getSchemas normalization', () => {
  let client: UnifiedSchemaClient;
  let mockApi: Pick<ApiClient, 'get'>;

  beforeEach(() => {
    mockApi = {
      // @ts-expect-error - we only mock the methods we use
      get: vi.fn()
    };
    // @ts-expect-error - pass partial mock
    client = new UnifiedSchemaClient(mockApi);
  });

  it('normalizes {data: [...]} to array', async () => {
    (mockApi.get as any).mockResolvedValue({ success: true, data: { data: [{ name: 'A' }, { name: 'B' }] } });
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


