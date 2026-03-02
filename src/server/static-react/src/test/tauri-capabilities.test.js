import { describe, it, expect } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const caps = JSON.parse(
  readFileSync(resolve(__dirname, '../../src-tauri/capabilities/default.json'), 'utf-8')
)

describe('Tauri capabilities', () => {
  it('includes dialog:allow-open required by FolderInput browse button', () => {
    expect(caps.permissions).toContain('dialog:allow-open')
  })
})
