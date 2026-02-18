import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';

const BASE_URL = process.env.FOLDDB_URL || 'http://localhost:9001';

async function callFoldDB(path: string, options?: RequestInit): Promise<string> {
  try {
    const res = await fetch(`${BASE_URL}${path}`, options);
    const data = await res.json();
    return JSON.stringify(data, null, 2);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return JSON.stringify({ error: `Failed to reach FoldDB: ${message}` }, null, 2);
  }
}

const server = new McpServer({
  name: 'folddb',
  version: '1.0.0',
});

// --- Status ---

server.tool(
  'folddb_status',
  'Check if FoldDB is running and healthy. Returns status, uptime, and version.',
  {},
  async () => ({
    content: [{ type: 'text', text: await callFoldDB('/api/system/status') }],
  })
);

// --- Schema List ---

server.tool(
  'folddb_schema_list',
  'List all schemas in FoldDB with their approval states.',
  {},
  async () => ({
    content: [{ type: 'text', text: await callFoldDB('/api/schemas') }],
  })
);

// --- Schema Get ---

server.tool(
  'folddb_schema_get',
  'Get the full definition of a specific schema by name.',
  { name: z.string().describe('Schema name to look up') },
  async ({ name }) => ({
    content: [{ type: 'text', text: await callFoldDB(`/api/schema/${encodeURIComponent(name)}`) }],
  })
);

// --- Structured Query ---

server.tool(
  'folddb_query',
  'Run a structured query against a schema. Returns matching records.',
  {
    schema_name: z.string().describe('Name of the schema to query'),
    fields: z.array(z.string()).describe('List of field names to return'),
    filter: z.any().optional().describe('Optional filter object, e.g. {"HashKey":"value"} or {"SampleN":10}'),
  },
  async ({ schema_name, fields, filter }) => {
    const body: Record<string, unknown> = { schema_name, fields };
    if (filter !== undefined) body.filter = filter;
    const text = await callFoldDB('/api/query', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    return { content: [{ type: 'text', text }] };
  }
);

// --- Native Index Search ---

server.tool(
  'folddb_search',
  'Search FoldDB native index for records matching a keyword. Returns schema/field/key matches.',
  { term: z.string().describe('Search keyword') },
  async ({ term }) => ({
    content: [{
      type: 'text',
      text: await callFoldDB(`/api/native-index/search?term=${encodeURIComponent(term)}`),
    }],
  })
);

// --- AI Agent Query ---

server.tool(
  'folddb_ask',
  'Ask a natural language question about data in FoldDB. An AI agent autonomously searches and queries to find the answer.',
  {
    query: z.string().describe('Natural language question about your data'),
    session_id: z.string().optional().describe('Optional session ID for conversation continuity'),
  },
  async ({ query, session_id }) => {
    const body: Record<string, unknown> = { query };
    if (session_id) body.session_id = session_id;
    const text = await callFoldDB('/api/llm-query/agent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    return { content: [{ type: 'text', text }] };
  }
);

// --- Ingest JSON ---

server.tool(
  'folddb_ingest',
  'Ingest JSON data into FoldDB with AI-powered schema detection. Returns immediately with a progress ID.',
  {
    data: z.record(z.any()).describe('JSON object to ingest'),
    source_file_name: z.string().optional().describe('Optional source filename for tracking'),
  },
  async ({ data, source_file_name }) => {
    const body: Record<string, unknown> = { data, auto_execute: true };
    if (source_file_name) body.source_file_name = source_file_name;
    const text = await callFoldDB('/api/ingestion/process', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    return { content: [{ type: 'text', text }] };
  }
);

// --- Mutate ---

server.tool(
  'folddb_mutate',
  'Create, update, or delete a record in FoldDB.',
  {
    schema: z.string().describe('Schema name'),
    fields_and_values: z.record(z.any()).describe('Field name to value mapping'),
    mutation_type: z.enum(['Create', 'Update', 'Delete']).describe('Type of mutation'),
    key_value: z.record(z.any()).optional().describe('Optional key fields for the record'),
  },
  async ({ schema, fields_and_values, mutation_type, key_value }) => {
    const body: Record<string, unknown> = {
      type: 'mutation',
      schema,
      fields_and_values,
      mutation_type,
    };
    if (key_value) body.key_value = key_value;
    const text = await callFoldDB('/api/mutation', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    return { content: [{ type: 'text', text }] };
  }
);

// --- Start server ---

const transport = new StdioServerTransport();
await server.connect(transport);
