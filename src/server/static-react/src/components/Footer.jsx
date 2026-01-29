function Footer() {
  return (
    <footer className="bg-terminal-lighter border-t border-terminal py-2 flex-shrink-0">
      <div className="max-w-7xl mx-auto px-6 text-center">
        <p className="text-terminal-dim text-xs font-mono">
          <span className="text-terminal-green">fold_db</span>
          <span className="text-terminal-dim mx-2">|</span>
          <span>node v1.0.0</span>
          <span className="text-terminal-dim mx-2">|</span>
          <span className="text-terminal-cyan">© {new Date().getFullYear()}</span>
        </p>
      </div>
    </footer>
  )
}

export default Footer