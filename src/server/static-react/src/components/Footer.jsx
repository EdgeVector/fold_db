import packageJson from '../../package.json'

function Footer() {
  return (
    <footer className="bg-surface border-t border-border px-8 py-2.5 flex-shrink-0 text-tertiary text-sm">
      <div className="flex items-center justify-between">
        <span>FoldDB v{packageJson.version}</span>
        <span>Local Mode</span>
      </div>
    </footer>
  )
}

export default Footer
