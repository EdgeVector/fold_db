function Footer() {
  return (
    <footer className="bg-white border-t border-gray-200 py-3">
      <div className="max-w-7xl mx-auto px-6 text-center">
        <p className="text-gray-600 text-sm">
          DataFold Node © {new Date().getFullYear()}
        </p>
      </div>
    </footer>
  )
}

export default Footer